use crate::api::TypedWebSocket;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Game, GameReq, Player, PlayerStatus};
use ferrogallic_shared::config::{
    WS_HEARTBEAT_INTERVAL, WS_RX_BUFFER_SHARED, WS_TX_BUFFER_BROADCAST, WS_TX_BUFFER_PER_CLIENT,
};
use ferrogallic_shared::domain::{Lobby, Nickname, UserId};
use futures::{SinkExt, StreamExt};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio::task::spawn;
use tokio::time::delay_for;

#[derive(Default)]
pub struct ActiveLobbies {
    tx_to_lobby: Mutex<HashMap<Lobby, mpsc::Sender<GameLoop>>>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct Epoch(NonZeroUsize);

impl Epoch {
    fn increment() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(1);

        let epoch = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(NonZeroUsize::new(epoch).unwrap())
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

pub async fn join_game(
    state: Arc<ActiveLobbies>,
    mut ws: TypedWebSocket<Game>,
) -> Result<(), Error> {
    let (lobby, nickname) = match ws.next().await {
        Some(Ok(GameReq::Join { lobby, nickname })) => (lobby, nickname),
        Some(Ok(m)) => return Err(anyhow!("Initial message was not Join: {:?}", m)),
        Some(Err(e)) => return Err(e.context("Failed to receive initial message")),
        None => return Err(anyhow!("WS closed before initial message")),
    };
    let user_id = nickname.user_id();
    let epoch = Epoch::increment();

    let (mut tx_to_lobby, mut rx_from_lobby, get_rx_broadcast) = loop {
        let mut tx_to_lobby = state
            .tx_to_lobby
            .lock()
            .await
            .entry(lobby.clone())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::channel(WS_RX_BUFFER_SHARED);
                spawn(run_game_loop(lobby.clone(), tx.clone(), rx));
                tx
            })
            .clone();

        let (tx, rx_from_lobby) = mpsc::channel(WS_TX_BUFFER_PER_CLIENT);
        let (give_rx_broadcast, get_rx_broadcast) = oneshot::channel();

        match tx_to_lobby
            .send(GameLoop::Connect(
                user_id,
                nickname.clone(),
                epoch,
                tx,
                give_rx_broadcast,
            ))
            .await
        {
            Ok(()) => break (tx_to_lobby, rx_from_lobby, get_rx_broadcast),
            Err(mpsc::error::SendError(_)) => {
                log::warn!("Player={} Lobby={} was shutdown, restart", nickname, lobby);
                state.tx_to_lobby.lock().await.remove(&lobby);
            }
        }
    };
    let mut tx_to_lobby_for_disconnect = tx_to_lobby.clone();

    let handle_messages = || async move {
        let mut rx_broadcast = get_rx_broadcast.await?;

        loop {
            select! {
                outbound = rx_from_lobby.recv() => match outbound {
                    Some(resp) => {
                        ws.send(&resp).await?;
                    }
                    None => {
                        log::info!("Player={} in Lobby={} dropped by game", nickname, lobby);
                        return Ok(());
                    }
                },
                outbound = rx_broadcast.recv() => match outbound {
                    Ok(broadcast) => match broadcast {
                        Broadcast::Everyone(resp) => ws.send(&resp).await?,
                        Broadcast::Exclude(uid, resp) if uid != user_id => ws.send(&resp).await?,
                        Broadcast::Exclude(_, resp) => {
                            log::trace!("Player={} in Lobby={} dropping excluded: {:?}", nickname, lobby, resp);
                        }
                    }
                    Err(broadcast::RecvError::Lagged(messages)) => {
                        log::warn!("Player={} in Lobby={} lagged {} messages behind", nickname, lobby, messages);
                        return Ok(());
                    }
                    Err(broadcast::RecvError::Closed) => {
                        log::info!("Player={} in Lobby={} dropped due to shutdown", nickname, lobby);
                        return Ok(());
                    }
                },
                inbound = ws.next() => match inbound {
                    Some(req) => match tx_to_lobby.send(GameLoop::Message(user_id, req?)).await {
                        Ok(()) => {}
                        Err(mpsc::error::SendError(_)) => {
                            log::info!("Player={} in Lobby={} dropped due to shutdown", nickname, lobby);
                            return Ok(());
                        }
                    }
                    None => {
                        log::info!("Player={} in Lobby={} disconnected", nickname, lobby);
                        return Ok(());
                    }
                },
            }
        }
    };

    let res = handle_messages().await;

    // if this fails, nothing we can do at this point, everyone is gone
    let _ = tx_to_lobby_for_disconnect
        .send(GameLoop::Disconnect(user_id, epoch))
        .await;

    res
}

enum GameLoop {
    Connect(
        UserId,
        Nickname,
        Epoch,
        mpsc::Sender<Game>,
        oneshot::Sender<broadcast::Receiver<Broadcast>>,
    ),
    Message(UserId, GameReq),
    Disconnect(UserId, Epoch),
    SendHeartbeat,
}

#[derive(Clone)]
enum Broadcast {
    Everyone(Game),
    Exclude(UserId, Game),
}

async fn run_game_loop(
    lobby: Lobby,
    tx_self: mpsc::Sender<GameLoop>,
    rx: mpsc::Receiver<GameLoop>,
) {
    match game_loop(&lobby, tx_self, rx).await {
        Ok(()) => log::info!("Lobby={} shutdown, no new connections", lobby),
        Err(e) => match e {
            GameLoopError::NoPlayers => log::info!("Lobby={} shutdown, no players left", lobby),
        },
    }
}

enum GameLoopError {
    NoPlayers,
}

impl From<broadcast::SendError<Broadcast>> for GameLoopError {
    fn from(_: broadcast::SendError<Broadcast>) -> Self {
        Self::NoPlayers
    }
}

async fn game_loop(
    lobby: &Lobby,
    tx_self: mpsc::Sender<GameLoop>,
    mut rx: mpsc::Receiver<GameLoop>,
) -> Result<(), GameLoopError> {
    log::info!("Lobby={} starting", lobby);

    let (tx_broadcast, _) = broadcast::channel(WS_TX_BUFFER_BROADCAST);

    let mut player_state = BTreeMap::new();
    let mut canvas_events = Vec::new();

    spawn({
        let lobby = lobby.clone();
        let mut tx_self = tx_self.clone();
        async move {
            loop {
                delay_for(WS_HEARTBEAT_INTERVAL).await;
                if let Err(e) = tx_self.send(GameLoop::SendHeartbeat).await {
                    log::info!("Lobby={} stopping heartbeat: {}", lobby, e);
                    return;
                }
            }
        }
    });

    loop {
        let msg = match rx.recv().await {
            Some(msg) => msg,
            None => return Ok(()),
        };
        match msg {
            GameLoop::Connect(user_id, nickname, epoch, mut tx, give_rx_broadcast) => {
                let onboard_player = || async {
                    give_rx_broadcast
                        .send(tx_broadcast.subscribe())
                        .map_err(|_| ())?;
                    tx.send(Game::CanvasBulk {
                        events: canvas_events.clone(),
                    })
                    .await
                    .map_err(|_| ())?;
                    Ok::<(), ()>(())
                };
                if let Err(()) = onboard_player().await {
                    log::warn!("Lobby={} Player={} failed onboarding", lobby, nickname);
                    continue;
                }
                match player_state.entry(user_id) {
                    Entry::Vacant(entry) => {
                        log::info!("Lobby={} Player={} Epoch={} join", lobby, nickname, epoch);
                        entry.insert(PlayerState {
                            nickname,
                            epoch,
                            tx: Some(tx),
                            score: 0,
                        });
                    }
                    Entry::Occupied(mut entry) => {
                        log::info!("Lobby={} Player={} Epoch={} reconn", lobby, nickname, epoch);
                        let state = entry.get_mut();
                        state.epoch = epoch;
                        state.tx = Some(tx);
                    }
                }
                tx_broadcast.send(Broadcast::Everyone(player_state_msg(&player_state)))?;
            }
            GameLoop::Message(user_id, req) => match req {
                GameReq::Canvas { event } => {
                    canvas_events.push(event);
                    tx_broadcast.send(Broadcast::Exclude(user_id, Game::Canvas { event }))?;
                }
                GameReq::Join { .. } => {
                    if let Some(state) = player_state.remove(&user_id) {
                        log::warn!("Lobby={} Player={} inval: {:?}", lobby, state.nickname, req);
                        tx_broadcast.send(Broadcast::Everyone(player_state_msg(&player_state)))?;
                    }
                }
            },
            GameLoop::Disconnect(user_id, epoch) => {
                if let Some(state) = player_state.get_mut(&user_id) {
                    if state.epoch == epoch {
                        state.tx = None;
                        tx_broadcast.send(Broadcast::Everyone(player_state_msg(&player_state)))?;
                    }
                }
            }
            GameLoop::SendHeartbeat => {
                tx_broadcast.send(Broadcast::Everyone(Game::Heartbeat))?;
            }
        }
    }
}

struct PlayerState {
    nickname: Nickname,
    epoch: Epoch,
    tx: Option<mpsc::Sender<Game>>,
    score: u32,
}

fn player_state_msg(player_state: &BTreeMap<UserId, PlayerState>) -> Game {
    Game::Players {
        players: player_state
            .iter()
            .map(|(user_id, state)| {
                (
                    *user_id,
                    Player {
                        nickname: state.nickname.clone(),
                        score: state.score,
                        status: match state.tx {
                            Some(_) => PlayerStatus::Connected,
                            None => PlayerStatus::Disconnected,
                        },
                    },
                )
            })
            .collect(),
    }
}
