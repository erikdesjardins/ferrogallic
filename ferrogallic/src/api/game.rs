use crate::api::TypedWebSocket;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Game, GameReq, Player, PlayerStatus};
use ferrogallic_shared::config::{
    REMOVE_DISCONNECTED_PLAYERS, WS_HEARTBEAT_INTERVAL, WS_RX_BUFFER_SHARED, WS_TX_BUFFER_BROADCAST,
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
    tx_lobby: Mutex<HashMap<Lobby, mpsc::Sender<GameLoop>>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    let (lobby, nick) = match ws.next().await {
        Some(Ok(GameReq::Join { lobby, nick })) => (lobby, nick),
        Some(Ok(m)) => return Err(anyhow!("Initial message was not Join: {:?}", m)),
        Some(Err(e)) => return Err(e.context("Failed to receive initial message")),
        None => return Err(anyhow!("WS closed before initial message")),
    };
    let user_id = nick.user_id();
    let epoch = Epoch::increment();

    let (mut tx_lobby, rx_onboard) = loop {
        let mut tx_lobby = state
            .tx_lobby
            .lock()
            .await
            .entry(lobby.clone())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::channel(WS_RX_BUFFER_SHARED);
                spawn(run_game_loop(lobby.clone(), tx.clone(), rx));
                tx
            })
            .clone();

        let (tx_onboard, rx_onboard) = oneshot::channel();

        match tx_lobby
            .send(GameLoop::Connect(user_id, epoch, nick.clone(), tx_onboard))
            .await
        {
            Ok(()) => break (tx_lobby, rx_onboard),
            Err(mpsc::error::SendError(_)) => {
                log::warn!("Player={} Lobby={} was shutdown, restart", nick, lobby);
                state.tx_lobby.lock().await.remove(&lobby);
            }
        }
    };
    let mut tx_lobby_for_disconnect = tx_lobby.clone();

    let handle_messages = || async move {
        let Onboarding {
            mut rx_broadcast,
            messages,
        } = rx_onboard.await?;

        for msg in messages {
            ws.send(&msg).await?;
        }

        loop {
            select! {
                outbound = rx_broadcast.recv() => match outbound {
                    Ok(broadcast) => match broadcast {
                        Broadcast::Everyone(resp) => ws.send(&resp).await?,
                        Broadcast::Exclude(uid, resp) if uid != user_id => ws.send(&resp).await?,
                        Broadcast::Kill(uid, ep) if uid == user_id && ep == epoch => {
                            log::info!("Player={} Lobby={} Epoch={} killed due to reconn", nick, lobby, epoch);
                            return Ok(());
                        }
                        Broadcast::Exclude(_, _) | Broadcast::Kill(_, _) => {
                            log::trace!("Player={} Lobby={} Epoch={} ignored: {:?}", nick, lobby, epoch, broadcast);
                        }
                    },
                    Err(broadcast::RecvError::Lagged(msgs)) => {
                        log::warn!("Player={} Lobby={} Epoch={} lagged {} messages", nick, lobby, epoch, msgs);
                        return Ok(());
                    }
                    Err(broadcast::RecvError::Closed) => {
                        log::info!("Player={} Lobby={} Epoch={} dropped on shutdown", nick, lobby, epoch);
                        return Ok(());
                    }
                },
                inbound = ws.next() => match inbound {
                    Some(Ok(req)) => match tx_lobby.send(GameLoop::Message(user_id, epoch, req)).await {
                        Ok(()) => {}
                        Err(mpsc::error::SendError(_)) => {
                            log::info!("Player={} Lobby={} Epoch={} dropped on shutdown", nick, lobby, epoch);
                            return Ok(());
                        }
                    }
                    Some(Err(e)) => {
                        log::info!("Player={} Lobby={} Epoch={} failed to receive: {}", nick, lobby, epoch, e);
                        return Ok(());
                    }
                    None => {
                        log::info!("Player={} Lobby={} Epoch={} disconnected", nick, lobby, epoch);
                        return Ok(());
                    }
                },
            }
        }
    };

    let res = handle_messages().await;

    // if this fails, nothing we can do at this point, everyone is gone
    let _ = tx_lobby_for_disconnect
        .send(GameLoop::Disconnect(user_id, epoch))
        .await;

    res
}

struct Onboarding {
    rx_broadcast: broadcast::Receiver<Broadcast>,
    messages: Vec<Game>,
}

enum GameLoop {
    Connect(UserId, Epoch, Nickname, oneshot::Sender<Onboarding>),
    Message(UserId, Epoch, GameReq),
    Disconnect(UserId, Epoch),
    Remove(UserId, Epoch),
    SendHeartbeat,
}

#[derive(Debug, Clone)]
enum Broadcast {
    Everyone(Game),
    Exclude(UserId, Game),
    Kill(UserId, Epoch),
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

#[derive(Debug)]
struct Connection {
    epoch: Epoch,
    player: Player,
}

async fn game_loop(
    lobby: &Lobby,
    tx_self: mpsc::Sender<GameLoop>,
    mut rx: mpsc::Receiver<GameLoop>,
) -> Result<(), GameLoopError> {
    log::info!("Lobby={} starting", lobby);

    let (tx_broadcast, _) = broadcast::channel(WS_TX_BUFFER_BROADCAST);

    let mut connections = BTreeMap::new();
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
            GameLoop::Connect(user_id, epoch, nick, tx_onboard) => {
                let onboarding = Onboarding {
                    rx_broadcast: tx_broadcast.subscribe(),
                    messages: vec![Game::CanvasBulk {
                        events: canvas_events.clone(),
                    }],
                };
                if let Err(_) = tx_onboard.send(onboarding) {
                    log::warn!("Lobby={} Player={} Epoch={} no onboard", lobby, nick, epoch);
                    continue;
                }
                match connections.entry(user_id) {
                    Entry::Vacant(entry) => {
                        log::info!("Lobby={} Player={} Epoch={} join", lobby, nick, epoch);
                        entry.insert(Connection {
                            epoch,
                            player: Player {
                                nick,
                                status: PlayerStatus::Connected,
                                score: 0,
                            },
                        });
                    }
                    Entry::Occupied(mut entry) => {
                        log::info!("Lobby={} Player={} Epoch={} reconn", lobby, nick, epoch);
                        let conn = entry.get_mut();
                        conn.epoch = epoch;
                        conn.player.status = PlayerStatus::Connected;
                    }
                }
                tx_broadcast.send(Broadcast::Everyone(players_msg(&connections)))?;
            }
            GameLoop::Message(user_id, epoch, req) => match connections.get(&user_id) {
                Some(conn) if conn.epoch == epoch => match req {
                    GameReq::Canvas { event } => {
                        canvas_events.push(event);
                        tx_broadcast.send(Broadcast::Exclude(user_id, Game::Canvas { event }))?;
                    }
                    GameReq::Join { .. } => {
                        if let Some(conn) = connections.remove(&user_id) {
                            let nick = &conn.player.nick;
                            log::warn!("Lobby={} Player={} invalid: {:?}", lobby, nick, req);
                            tx_broadcast.send(Broadcast::Everyone(players_msg(&connections)))?;
                        }
                    }
                },
                _ => {
                    tx_broadcast.send(Broadcast::Kill(user_id, epoch))?;
                }
            },
            GameLoop::Disconnect(user_id, epoch) => {
                if let Some(conn) = connections.get_mut(&user_id) {
                    if conn.epoch == epoch {
                        conn.player.status = PlayerStatus::Disconnected;
                        tx_broadcast.send(Broadcast::Everyone(players_msg(&connections)))?;
                        spawn({
                            let mut tx_self = tx_self.clone();
                            async move {
                                delay_for(REMOVE_DISCONNECTED_PLAYERS).await;
                                let _ = tx_self.send(GameLoop::Remove(user_id, epoch)).await;
                            }
                        });
                    }
                }
            }
            GameLoop::Remove(user_id, epoch) => {
                if let Entry::Occupied(entry) = connections.entry(user_id) {
                    if entry.get().epoch == epoch {
                        let conn = entry.remove();
                        log::warn!("Lobby={} Player={} removed", lobby, conn.player.nick);
                        tx_broadcast.send(Broadcast::Everyone(players_msg(&connections)))?;
                    }
                }
            }
            GameLoop::SendHeartbeat => {
                tx_broadcast.send(Broadcast::Everyone(Game::Heartbeat))?;
            }
        }
    }
}

fn players_msg(connections: &BTreeMap<UserId, Connection>) -> Game {
    Game::Players {
        players: connections
            .iter()
            .map(|(user_id, conn)| (*user_id, conn.player.clone()))
            .collect(),
    }
}
