use crate::api::TypedWebSocket;
use crate::words;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Canvas, Game, GameReq, GameState, Player, PlayerStatus};
use ferrogallic_shared::config::{
    NUMBER_OF_WORDS_TO_CHOOSE, REMOVE_DISCONNECTED_PLAYERS, WS_HEARTBEAT_INTERVAL,
    WS_RX_BUFFER_SHARED, WS_TX_BUFFER_BROADCAST,
};
use ferrogallic_shared::domain::{Guess, Lobby, Nickname, UserId};
use futures::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::cell::Cell;
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
                            log::info!("Player={} Lobby={} Epoch={} killed", nick, lobby, epoch);
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

#[test]
fn broadcast_size() {
    assert_eq!(std::mem::size_of::<Broadcast>(), 48);
}

async fn run_game_loop(
    lobby: Lobby,
    tx_self: mpsc::Sender<GameLoop>,
    rx: mpsc::Receiver<GameLoop>,
) {
    match game_loop(&lobby, tx_self, rx).await {
        Ok(()) => log::info!("Lobby={} shutdown, no new connections", lobby),
        Err(e) => match e {
            GameLoopError::NoPlayers => {
                log::info!("Lobby={} shutdown, no players left", lobby);
            }
            GameLoopError::NoConnectionsDuringStateChange => {
                log::info!(
                    "Lobby={} shutdown, no connections during state change",
                    lobby
                );
            }
        },
    }
}

enum GameLoopError {
    NoPlayers,
    NoConnectionsDuringStateChange,
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

enum Transition {
    ChoosingWords { previously_drawing: Option<UserId> },
}

async fn game_loop(
    lobby: &Lobby,
    tx_self: mpsc::Sender<GameLoop>,
    mut rx: mpsc::Receiver<GameLoop>,
) -> Result<(), GameLoopError> {
    log::info!("Lobby={} starting", lobby);

    let (tx_broadcast, _) = broadcast::channel(WS_TX_BUFFER_BROADCAST);

    let mut connections = Invalidate::new(BTreeMap::new());
    let mut game_state = Invalidate::new(GameState::default());
    let mut canvas_events = Vec::new();
    let mut guesses = Vec::new();

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
                    messages: vec![
                        Game::Game {
                            state: Arc::new(game_state.read().clone()),
                        },
                        Game::GuessBulk {
                            guesses: guesses.clone(),
                        },
                        Game::CanvasBulk {
                            events: canvas_events.clone(),
                        },
                    ],
                };
                if let Err(_) = tx_onboard.send(onboarding) {
                    log::warn!("Lobby={} Player={} Epoch={} no onboard", lobby, nick, epoch);
                    continue;
                }
                match connections.write().entry(user_id) {
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
            }
            GameLoop::Message(user_id, epoch, req) => match connections.read().get(&user_id) {
                Some(conn) if conn.epoch == epoch => match req {
                    GameReq::Canvas { event } => {
                        canvas_events.push(event);
                        tx_broadcast.send(Broadcast::Exclude(user_id, Game::Canvas { event }))?;
                    }
                    GameReq::Choose { word } => match game_state.read() {
                        GameState::ChoosingWords { choosing, words }
                            if *choosing == user_id && words.contains(&word) =>
                        {
                            *game_state.write() = GameState::Drawing {
                                drawing: *choosing,
                                correct: Default::default(),
                                word,
                            };
                            tx_broadcast.send(Broadcast::Everyone(Game::Canvas {
                                event: Canvas::Clear,
                            }))?;
                        }
                        gs => {
                            let nick = &conn.player.nick;
                            log::warn!("Lobby={} Player={} invalid choose: {:?}", lobby, nick, gs);
                            tx_broadcast.send(Broadcast::Kill(user_id, epoch))?;
                        }
                    },
                    GameReq::Guess { guess } => {
                        let guess = match game_state.read() {
                            GameState::WaitingToStart { .. } => match guess.as_ref() {
                                "start" => {
                                    if let GameState::WaitingToStart { starting } =
                                        game_state.write()
                                    {
                                        *starting = true;
                                    }
                                    Guess::System("Starting game...".into())
                                }
                                _ => Guess::Message(guess),
                            },
                            GameState::ChoosingWords { .. } => Guess::Message(guess),
                            GameState::Drawing {
                                drawing,
                                correct,
                                word,
                            } => {
                                if *drawing == user_id || correct.contains(&user_id) {
                                    Guess::Message(guess)
                                } else if &guess == word {
                                    if let GameState::Drawing { correct, .. } = game_state.write() {
                                        correct.push(user_id);
                                    }
                                    Guess::Correct(user_id)
                                } else {
                                    Guess::Guess(guess)
                                }
                            }
                        };
                        guesses.push(guess.clone());
                        tx_broadcast.send(Broadcast::Everyone(Game::Guess { guess }))?;
                    }
                    GameReq::Join { .. } => {
                        let nick = &conn.player.nick;
                        log::warn!("Lobby={} Player={} invalid: {:?}", lobby, nick, req);
                        tx_broadcast.send(Broadcast::Kill(user_id, epoch))?;
                    }
                },
                _ => {
                    tx_broadcast.send(Broadcast::Kill(user_id, epoch))?;
                }
            },
            GameLoop::Disconnect(user_id, epoch) => {
                if let Some(conn) = connections.write().get_mut(&user_id) {
                    if conn.epoch == epoch {
                        conn.player.status = PlayerStatus::Disconnected;
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
                if let Entry::Occupied(entry) = connections.write().entry(user_id) {
                    if entry.get().epoch == epoch {
                        let conn = entry.remove();
                        log::warn!("Lobby={} Player={} removed", lobby, conn.player.nick);
                    }
                }
            }
            GameLoop::SendHeartbeat => {
                tx_broadcast.send(Broadcast::Everyone(Game::Heartbeat))?;
            }
        }

        let transition = if game_state.is_changed() {
            match game_state.read() {
                GameState::WaitingToStart { starting: true } => Some(Transition::ChoosingWords {
                    previously_drawing: None,
                }),
                GameState::WaitingToStart { starting: false } => None,
                GameState::ChoosingWords {
                    choosing: _,
                    words: _,
                } => None,
                GameState::Drawing {
                    drawing,
                    correct,
                    word: _,
                } => {
                    if connections
                        .read()
                        .keys()
                        .all(|uid| drawing == uid || correct.contains(uid))
                    {
                        Some(Transition::ChoosingWords {
                            previously_drawing: Some(*drawing),
                        })
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };

        match transition {
            Some(Transition::ChoosingWords { previously_drawing }) => {
                let choosing = connections
                    .read()
                    .keys()
                    // first player after the previous drawer...
                    .skip_while(|uid| Some(**uid) != previously_drawing)
                    .nth(1)
                    // ...or the first player in the list
                    .or_else(|| connections.read().keys().next());
                let choosing = match choosing {
                    Some(choosing) => *choosing,
                    None => return Err(GameLoopError::NoConnectionsDuringStateChange),
                };
                let words = words::GAME
                    .choose_multiple(&mut thread_rng(), NUMBER_OF_WORDS_TO_CHOOSE)
                    .map(|&s| s.into())
                    .collect();
                *game_state.write() = GameState::ChoosingWords { choosing, words };
            }
            None => {}
        }

        if let Some(connections) = connections.reset_if_changed() {
            tx_broadcast.send(Broadcast::Everyone(Game::Players {
                players: Arc::new(
                    connections
                        .iter()
                        .map(|(user_id, conn)| (*user_id, conn.player.clone()))
                        .collect(),
                ),
            }))?;
        }
        if let Some(game_state) = game_state.reset_if_changed() {
            tx_broadcast.send(Broadcast::Everyone(Game::Game {
                state: Arc::new(game_state.clone()),
            }))?;
        }
    }
}

struct Invalidate<T> {
    value: T,
    changed: Cell<bool>,
}

impl<T> Invalidate<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            changed: Cell::new(true),
        }
    }

    fn read(&self) -> &T {
        &self.value
    }

    fn write(&mut self) -> &mut T {
        self.changed.set(true);
        &mut self.value
    }

    fn is_changed(&self) -> bool {
        self.changed.get()
    }

    fn reset_if_changed(&self) -> Option<&T> {
        if self.changed.replace(false) {
            Some(&self.value)
        } else {
            None
        }
    }
}
