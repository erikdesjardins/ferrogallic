use crate::api::TypedWebSocket;
use crate::words;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Canvas, Game, GameReq, GameState, Player, PlayerStatus};
use ferrogallic_shared::config::{
    CLOSE_GUESS_LEVENSHTEIN, GUESS_SECONDS, NOTIFY_TIME_REMAINING_AT, NUMBER_OF_WORDS_TO_CHOOSE,
    PERFECT_GUESS_SCORE, TIMER_TICK_SECONDS, WS_RX_BUFFER_SHARED, WS_TX_BUFFER_BROADCAST,
};
use ferrogallic_shared::domain::{Epoch, Guess, Lobby, Lowercase, Nickname, UserId};
use futures::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::cell::Cell;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use strsim::levenshtein;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio::task::spawn;
use tokio::time::interval;

#[derive(Default)]
pub struct ActiveLobbies {
    tx_lobby: Mutex<HashMap<CaseInsensitiveLobby, mpsc::Sender<GameLoop>>>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct CaseInsensitiveLobby(Box<str>);

impl CaseInsensitiveLobby {
    fn new(lobby: &Lobby) -> Self {
        Self(lobby.to_ascii_lowercase().into_boxed_str())
    }
}

pub async fn join_game(
    state: Arc<ActiveLobbies>,
    mut ws: TypedWebSocket<Game>,
) -> Result<(), Error> {
    let (lobby, nick) = match ws.next().await {
        Some(Ok(GameReq::Join(lobby, nick))) => (lobby, nick),
        Some(Ok(m)) => return Err(anyhow!("Initial message was not Join: {:?}", m)),
        Some(Err(e)) => return Err(e.context("Failed to receive initial message")),
        None => return Err(anyhow!("WS closed before initial message")),
    };
    let user_id = nick.user_id();
    let epoch = Epoch::next();

    let (mut tx_lobby, rx_onboard) = loop {
        let mut tx_lobby = state
            .tx_lobby
            .lock()
            .await
            .entry(CaseInsensitiveLobby::new(&lobby))
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
                state
                    .tx_lobby
                    .lock()
                    .await
                    .remove(&CaseInsensitiveLobby::new(&lobby));
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
                        Broadcast::Only(uid, resp) if uid == user_id => ws.send(&resp).await?,
                        Broadcast::Kill(uid, ep) if uid == user_id && ep == epoch => {
                            log::info!("Player={} Lobby={} Epoch={} killed", nick, lobby, epoch);
                            return Ok(());
                        }
                        Broadcast::Exclude(_, _) | Broadcast::Only(_, _)  | Broadcast::Kill(_, _) => {
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
    SecondsElapsed(u8),
}

#[test]
fn gameloop_size() {
    assert_eq!(std::mem::size_of::<GameLoop>(), 64);
}

#[derive(Debug, Clone)]
enum Broadcast {
    Everyone(Game),
    Exclude(UserId, Game),
    Only(UserId, Game),
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
    log::info!("Lobby={} starting", lobby);

    spawn({
        let lobby = lobby.clone();
        let mut tx_self_sending_and_receiving_on_same_task_can_deadlock = tx_self;
        async move {
            let mut interval = interval(Duration::from_secs(u64::from(TIMER_TICK_SECONDS)));
            loop {
                interval.tick().await;
                if let Err(e) = tx_self_sending_and_receiving_on_same_task_can_deadlock
                    .send(GameLoop::SecondsElapsed(TIMER_TICK_SECONDS))
                    .await
                {
                    log::info!("Lobby={} stopping timer: {}", lobby, e);
                    return;
                }
            }
        }
    });

    match game_loop(&lobby, rx).await {
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

enum Transition {
    ChoosingWords { previously_drawing: Option<UserId> },
}

async fn game_loop(lobby: &Lobby, mut rx: mpsc::Receiver<GameLoop>) -> Result<(), GameLoopError> {
    let (tx, _) = broadcast::channel(WS_TX_BUFFER_BROADCAST);

    let mut players = Invalidate::new(Arc::new(BTreeMap::new()));
    let mut game_state = Invalidate::new(Arc::new(GameState::default()));
    let mut canvas_events = Vec::new();
    let mut guesses = Vec::new();

    loop {
        let msg = match rx.recv().await {
            Some(msg) => msg,
            None => return Ok(()),
        };
        match msg {
            GameLoop::Connect(user_id, epoch, nick, tx_onboard) => {
                let onboarding = Onboarding {
                    rx_broadcast: tx.subscribe(),
                    messages: vec![
                        Game::Game(game_state.read().clone()),
                        Game::GuessBulk(guesses.clone()),
                        Game::CanvasBulk(canvas_events.clone()),
                    ],
                };
                if let Err(_) = tx_onboard.send(onboarding) {
                    log::warn!("Lobby={} Player={} Epoch={} no onboard", lobby, nick, epoch);
                    continue;
                }
                match Arc::make_mut(players.write()).entry(user_id) {
                    Entry::Vacant(entry) => {
                        log::info!("Lobby={} Player={} Epoch={} join", lobby, nick, epoch);
                        entry.insert(Player {
                            nick,
                            epoch,
                            status: PlayerStatus::Connected,
                            score: 0,
                        });
                    }
                    Entry::Occupied(mut entry) => {
                        log::info!("Lobby={} Player={} Epoch={} reconn", lobby, nick, epoch);
                        let player = entry.get_mut();
                        player.epoch = epoch;
                        player.status = PlayerStatus::Connected;
                    }
                }
            }
            GameLoop::Message(user_id, epoch, req) => match players.read().get(&user_id) {
                Some(player) if player.epoch == epoch => match req {
                    GameReq::Canvas(event) => {
                        canvas_events.push(event);
                        tx.send(Broadcast::Exclude(user_id, Game::Canvas(event)))?;
                    }
                    GameReq::Choose(word) => match game_state.read().as_ref() {
                        GameState::ChoosingWords { choosing, words }
                            if *choosing == user_id && words.contains(&word) =>
                        {
                            let drawing = *choosing;
                            *Arc::make_mut(game_state.write()) = GameState::Drawing {
                                drawing,
                                correct_scores: Default::default(),
                                word,
                                seconds_remaining: GUESS_SECONDS,
                            };
                            tx.send(Broadcast::Everyone(Game::Guess(Guess::NowDrawing(drawing))))?;
                            canvas_events.clear();
                            tx.send(Broadcast::Everyone(Game::Canvas(Canvas::Clear)))?;
                        }
                        gs => {
                            let nick = &player.nick;
                            log::warn!("Lobby={} Player={} invalid choose: {:?}", lobby, nick, gs);
                            tx.send(Broadcast::Kill(user_id, epoch))?;
                        }
                    },
                    GameReq::Guess(guess) => {
                        let guess = match game_state.read().as_ref() {
                            GameState::WaitingToStart { .. } => match guess.as_ref() {
                                "start" => {
                                    if let GameState::WaitingToStart { starting } =
                                        Arc::make_mut(game_state.write())
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
                                } else if guess == *word {
                                    let seconds_remaining = *seconds_remaining;
                                    if let GameState::Drawing { correct_scores, .. } =
                                        Arc::make_mut(game_state.write())
                                    {
                                        correct_scores
                                            .insert(user_id, guesser_score(seconds_remaining));
                                    }
                                    Guess::Correct(user_id)
                                } else {
                                    if levenshtein(&guess, word) <= CLOSE_GUESS_LEVENSHTEIN {
                                        tx.send(Broadcast::Only(
                                            user_id,
                                            Game::Guess(Guess::CloseGuess(guess.clone())),
                                        ))?;
                                    }

                                    Guess::Guess(user_id, guess)
                                }
                            }
                        };
                        guesses.push(guess.clone());
                        tx.send(Broadcast::Everyone(Game::Guess(guess)))?;
                    }
                    GameReq::Remove(remove_uid, remove_epoch) => {
                        if let Entry::Occupied(entry) =
                            Arc::make_mut(players.write()).entry(remove_uid)
                        {
                            if entry.get().epoch == remove_epoch {
                                let removed = entry.remove();
                                log::info!("Lobby={} Player={} removed", lobby, removed.nick);
                            }
                        }
                    }
                    GameReq::Join(..) => {
                        log::warn!("Lobby={} Player={} invalid: {:?}", lobby, player.nick, req);
                        tx.send(Broadcast::Kill(user_id, epoch))?;
                    }
                },
                _ => {
                    tx.send(Broadcast::Kill(user_id, epoch))?;
                }
            },
            GameLoop::Disconnect(user_id, epoch) => {
                if let Some(player) = Arc::make_mut(players.write()).get_mut(&user_id) {
                    if player.epoch == epoch {
                        player.status = PlayerStatus::Disconnected;
                    }
                }
            }
            GameLoop::SecondsElapsed(seconds) => match game_state.read().as_ref() {
                GameState::WaitingToStart { .. } | GameState::ChoosingWords { .. } => {
                    tx.send(Broadcast::Everyone(Game::Heartbeat))?;
                }
                GameState::Drawing { .. } => {
                    if let GameState::Drawing {
                        seconds_remaining, ..
                    } = Arc::make_mut(game_state.write())
                    {
                        *seconds_remaining = seconds_remaining.saturating_sub(seconds);
                        if NOTIFY_TIME_REMAINING_AT.contains(seconds_remaining) {
                            tx.send(Broadcast::Everyone(Game::Guess(Guess::SecondsLeft(
                                *seconds_remaining,
                            ))))?;
                        }
                    }
                }
            },
        }

        let transition = if game_state.is_changed() {
            match game_state.read().as_ref() {
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
                    word,
                    seconds_remaining,
                } => {
                    if *seconds_remaining == 0
                        || players
                            .read()
                            .keys()
                            .all(|uid| drawing == uid || correct_scores.contains_key(uid))
                    {
                        if *seconds_remaining == 0 {
                            let guess = Guess::TimeExpired(word.clone());
                            guesses.push(guess.clone());
                            tx.send(Broadcast::Everyone(Game::Guess(guess)))?;
                        }
                        let players = Arc::make_mut(players.write());
                        for (&user_id, &score) in correct_scores {
                            players
                                .entry(user_id)
                                .and_modify(|player| player.score += score);
                            let guess = Guess::EarnedPoints(user_id, score);
                            guesses.push(guess.clone());
                            tx.send(Broadcast::Everyone(Game::Guess(guess)))?;
                        }
                        let player_count = players.len() as u32;
                        if let Some(drawer) = players.get_mut(drawing) {
                            drawer.score +=
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
                let choosing = players
                    .read()
                    .keys()
                    // first player after the previous drawer...
                    .skip_while(|uid| Some(**uid) != previously_drawing)
                    .nth(1)
                    // ...or the first player in the list
                    .or_else(|| players.read().keys().next());
                let choosing = match choosing {
                    Some(choosing) => *choosing,
                    None => return Err(GameLoopError::NoConnectionsDuringStateChange),
                };
                let words = words::GAME
                    .choose_multiple(&mut thread_rng(), NUMBER_OF_WORDS_TO_CHOOSE)
                    .copied()
                    .map(Lowercase::new)
                    .collect();
                *Arc::make_mut(game_state.write()) = GameState::ChoosingWords { choosing, words };
                let guess = Guess::NowChoosing(choosing);
                guesses.push(guess.clone());
                tx.send(Broadcast::Everyone(Game::Guess(guess)))?;
            }
            None => {}
        }

        if let Some(players) = players.reset_if_changed() {
            tx.send(Broadcast::Everyone(Game::Players(players.clone())))?;
        }
        if let Some(game_state) = game_state.reset_if_changed() {
            tx.send(Broadcast::Everyone(Game::Game(game_state.clone())))?;
        }
    }
}

fn guesser_score(seconds_remaining: u8) -> u32 {
    u32::from(seconds_remaining) * PERFECT_GUESS_SCORE / u32::from(GUESS_SECONDS)
}

fn drawer_score(scores: impl Iterator<Item = u32>, player_count: u32) -> u32 {
    scores
        .sum::<u32>()
        .checked_div(player_count - 1)
        .unwrap_or(0)
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
