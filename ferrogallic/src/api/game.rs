use crate::api::TypedWebSocket;
use crate::words;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Canvas, Game, GameReq, GameState, Player, PlayerStatus};
use ferrogallic_shared::config::{
    GUESS_SECONDS, NUMBER_OF_WORDS_TO_CHOOSE, PERFECT_GUESS_SCORE, REMOVE_DISCONNECTED_PLAYERS,
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
use std::time::Duration;
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
    OneSecondElapsed,
}

#[derive(Debug, Clone)]
enum Broadcast {
    Everyone(Game),
    Exclude(UserId, Game),
    Kill(UserId, Epoch),
}

#[test]
fn broadcast_size() {
    assert_eq!(std::mem::size_of::<Broadcast>(), 56);
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
                delay_for(Duration::from_secs(1)).await;
                if let Err(e) = tx_self.send(GameLoop::OneSecondElapsed).await {
                    log::info!("Lobby={} stopping timer: {}", lobby, e);
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
                        Game::Game(Arc::new(game_state.read().clone())),
                        Game::GuessBulk(guesses.clone()),
                        Game::CanvasBulk(canvas_events.clone()),
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
                        tx_broadcast.send(Broadcast::Exclude(user_id, Game::Canvas(event)))?;
                    }
                    GameReq::Choose { word } => match game_state.read() {
                        GameState::ChoosingWords { choosing, words }
                            if *choosing == user_id && words.contains(&word) =>
                        {
                            *game_state.write() = GameState::Drawing {
                                drawing: *choosing,
                                correct_scores: Default::default(),
                                word,
                                seconds_remaining: GUESS_SECONDS,
                            };
                            canvas_events.clear();
                            tx_broadcast.send(Broadcast::Everyone(Game::Canvas(Canvas::Clear)))?;
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
                                _ => Guess::Message(user_id, guess),
                            },
                            GameState::ChoosingWords { .. } => Guess::Message(user_id, guess),
                            GameState::Drawing {
                                drawing,
                                correct_scores,
                                word,
                                seconds_remaining,
                            } => {
                                if *drawing == user_id || correct_scores.contains_key(&user_id) {
                                    Guess::Message(user_id, guess)
                                } else if guess.eq_ignore_ascii_case(word) {
                                    let seconds_remaining = *seconds_remaining;
                                    if let GameState::Drawing { correct_scores, .. } =
                                        game_state.write()
                                    {
                                        correct_scores
                                            .insert(user_id, guesser_score(seconds_remaining));
                                    }
                                    Guess::Correct(user_id)
                                } else {
                                    Guess::Guess(user_id, guess)
                                }
                            }
                        };
                        guesses.push(guess.clone());
                        tx_broadcast.send(Broadcast::Everyone(Game::Guess(guess)))?;
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
            GameLoop::OneSecondElapsed => match game_state.read() {
                GameState::WaitingToStart { .. } | GameState::ChoosingWords { .. } => {
                    tx_broadcast.send(Broadcast::Everyone(Game::Heartbeat))?;
                }
                GameState::Drawing { .. } => {
                    if let GameState::Drawing {
                        seconds_remaining, ..
                    } = game_state.write()
                    {
                        *seconds_remaining = seconds_remaining.saturating_sub(1);
                    }
                }
            },
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
                    correct_scores,
                    word: _,
                    seconds_remaining,
                } => {
                    if *seconds_remaining == 0
                        || connections
                            .read()
                            .keys()
                            .all(|uid| drawing == uid || correct_scores.contains_key(uid))
                    {
                        if *seconds_remaining == 0 {
                            tx_broadcast
                                .send(Broadcast::Everyone(Game::Guess(Guess::TimeExpired)))?;
                        }
                        let connections = connections.write();
                        for (&user_id, &score) in correct_scores {
                            connections
                                .entry(user_id)
                                .and_modify(|conn| conn.player.score += score);
                            tx_broadcast.send(Broadcast::Everyone(Game::Guess(
                                Guess::EarnedPoints(user_id, score),
                            )))?;
                        }
                        let player_count = connections.len() as u32;
                        if let Some(drawer) = connections.get_mut(drawing) {
                            drawer.player.score +=
                                drawer_score(correct_scores.values().copied(), player_count);
                        }
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
                tx_broadcast.send(Broadcast::Everyone(Game::Guess(Guess::NowChoosing(
                    choosing,
                ))))?;
            }
            None => {}
        }

        if let Some(connections) = connections.reset_if_changed() {
            tx_broadcast.send(Broadcast::Everyone(Game::Players(Arc::new(
                connections
                    .iter()
                    .map(|(user_id, conn)| (*user_id, conn.player.clone()))
                    .collect(),
            ))))?;
        }
        if let Some(game_state) = game_state.reset_if_changed() {
            tx_broadcast.send(Broadcast::Everyone(Game::Game(Arc::new(
                game_state.clone(),
            ))))?;
        }
    }
}

fn guesser_score(seconds_remaining: u8) -> u32 {
    u32::from(seconds_remaining) * PERFECT_GUESS_SCORE / u32::from(GUESS_SECONDS)
}

fn drawer_score(scores: impl Iterator<Item = u32>, player_count: u32) -> u32 {
    scores.sum::<u32>() / (player_count - 1)
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
