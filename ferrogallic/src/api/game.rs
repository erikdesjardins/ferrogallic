use crate::api::TypedWebSocket;
use crate::words;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Canvas, Game, GameReq, GameState, Player, PlayerStatus};
use ferrogallic_shared::config::{
    CLOSE_GUESS_LEVENSHTEIN, GUESS_SECONDS, HEARTBEAT_SECONDS, NUMBER_OF_WORDS_TO_CHOOSE,
    PERFECT_GUESS_SCORE, RX_SHARED_BUFFER, TX_BROADCAST_BUFFER, TX_SELF_DELAYED_BUFFER,
};
use ferrogallic_shared::domain::{Epoch, Guess, Lobby, Lowercase, Nickname, UserId};
use futures::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::cell::Cell;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use strsim::levenshtein;
use time::OffsetDateTime;
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio::task::spawn;
use tokio::time::{interval, DelayQueue, Duration, Instant};

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
                let (tx, rx) = mpsc::channel(RX_SHARED_BUFFER);
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
    Connect(UserId, Epoch<UserId>, Nickname, oneshot::Sender<Onboarding>),
    Message(UserId, Epoch<UserId>, GameReq),
    Disconnect(UserId, Epoch<UserId>),
    Heartbeat,
    GameEnd(Epoch<GameState>),
}

#[derive(Debug, Clone)]
enum Broadcast {
    Everyone(Game),
    Exclude(UserId, Game),
    Only(UserId, Game),
    Kill(UserId, Epoch<UserId>),
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

    let (tx_self_delayed, mut rx_self_delayed) = mpsc::channel(TX_SELF_DELAYED_BUFFER);

    spawn({
        let lobby = lobby.clone();
        let mut tx_self_sending_and_receiving_on_same_task_can_deadlock = tx_self;
        async move {
            let mut delay = DelayQueue::new();
            let mut heartbeat = interval(Duration::from_secs(HEARTBEAT_SECONDS));
            loop {
                let msg = select! {
                    to_delay = rx_self_delayed.recv() => match to_delay {
                        Some((to_delay, instant)) => {
                            delay.insert_at(to_delay, instant);
                            continue;
                        }
                        None => {
                            log::info!("Lobby={} stopping timer: game loop disconnected", lobby);
                            return;
                        }
                    },
                    Some(delayed) = delay.next() => match delayed {
                        Ok(delayed) => delayed.into_inner(),
                        Err(e) => {
                            log::error!("Lobby={} stopping timer due to error: {}", lobby, e);
                            return;
                        }
                    },
                    now = heartbeat.tick() => {
                        let _: Instant = now;
                        GameLoop::Heartbeat
                    }
                };
                if let Err(e) = tx_self_sending_and_receiving_on_same_task_can_deadlock
                    .send(msg)
                    .await
                {
                    log::info!("Lobby={} stopping timer: {}", lobby, e);
                    return;
                }
            }
        }
    });

    match game_loop(&lobby, tx_self_delayed, rx).await {
        Ok(()) => log::info!("Lobby={} shutdown, no new connections", lobby),
        Err(e) => match e {
            GameLoopError::NoPlayers => {
                log::info!("Lobby={} shutdown: no players left", lobby);
            }
            GameLoopError::NoConnectionsDuringStateChange => {
                log::info!("Lobby={} shutdown: no conns during state change", lobby);
            }
            GameLoopError::DelayRecvGone => {
                log::error!("Lobby={} shutdown: delay receiver gone", lobby);
            }
        },
    }
}

enum GameLoopError {
    NoPlayers,
    NoConnectionsDuringStateChange,
    DelayRecvGone,
}

impl From<broadcast::SendError<Broadcast>> for GameLoopError {
    fn from(_: broadcast::SendError<Broadcast>) -> Self {
        Self::NoPlayers
    }
}

impl From<mpsc::error::SendError<(GameLoop, Instant)>> for GameLoopError {
    fn from(_: mpsc::error::SendError<(GameLoop, Instant)>) -> Self {
        Self::DelayRecvGone
    }
}

enum Transition {
    ChoosingWords { previously_drawing: Option<UserId> },
}

async fn game_loop(
    lobby: &Lobby,
    mut tx_self_delayed: mpsc::Sender<(GameLoop, Instant)>,
    mut rx: mpsc::Receiver<GameLoop>,
) -> Result<(), GameLoopError> {
    let (tx, _) = broadcast::channel(TX_BROADCAST_BUFFER);

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
            GameLoop::Message(user_id, epoch, req) => {
                let players_ = players.read().as_ref();
                match players_.get(&user_id) {
                    Some(player) if player.epoch == epoch => match req {
                        GameReq::Canvas(event) => {
                            canvas_events.push(event);
                            tx.send(Broadcast::Exclude(user_id, Game::Canvas(event)))?;
                        }
                        GameReq::Choose(word) => {
                            let game_state_ = game_state.read().as_ref();
                            match game_state_ {
                                GameState::ChoosingWords { choosing, words }
                                    if *choosing == user_id && words.contains(&word) =>
                                {
                                    let drawing = *choosing;
                                    let game_epoch = Epoch::next();
                                    let started = OffsetDateTime::now_utc();
                                    let will_end =
                                        Instant::now() + Duration::from_secs(GUESS_SECONDS.into());
                                    *Arc::make_mut(game_state.write()) = GameState::Drawing {
                                        drawing,
                                        correct_scores: Default::default(),
                                        word,
                                        epoch: game_epoch,
                                        started,
                                        timed_out: false,
                                    };
                                    tx_self_delayed
                                        .send((GameLoop::GameEnd(game_epoch), will_end))
                                        .await?;
                                    let guess = Guess::NowDrawing(drawing);
                                    guesses.push(guess.clone());
                                    tx.send(Broadcast::Everyone(Game::Guess(guess)))?;
                                    canvas_events.clear();
                                    tx.send(Broadcast::Everyone(Game::Canvas(Canvas::Clear)))?;
                                }
                                gs => {
                                    let nick = &player.nick;
                                    log::warn!(
                                        "Lobby={} Player={} invalid choose: {:?}",
                                        lobby,
                                        nick,
                                        gs
                                    );
                                    tx.send(Broadcast::Kill(user_id, epoch))?;
                                }
                            }
                        }
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
                                    epoch: _,
                                    started,
                                    timed_out: _,
                                } => {
                                    if *drawing == user_id || correct_scores.contains_key(&user_id)
                                    {
                                        Guess::Message(user_id, guess)
                                    } else if guess == *word {
                                        let elapsed = OffsetDateTime::now_utc() - *started;
                                        if let GameState::Drawing { correct_scores, .. } =
                                            Arc::make_mut(game_state.write())
                                        {
                                            correct_scores.insert(user_id, guesser_score(elapsed));
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
                }
            }
            GameLoop::Disconnect(user_id, epoch) => {
                if let Some(player) = Arc::make_mut(players.write()).get_mut(&user_id) {
                    if player.epoch == epoch {
                        player.status = PlayerStatus::Disconnected;
                    }
                }
            }
            GameLoop::Heartbeat => {
                tx.send(Broadcast::Everyone(Game::Heartbeat))?;
            }
            GameLoop::GameEnd(ended_epoch) => {
                if let GameState::Drawing { epoch, .. } = game_state.read().as_ref() {
                    if *epoch == ended_epoch {
                        if let GameState::Drawing { timed_out, .. } =
                            Arc::make_mut(game_state.write())
                        {
                            *timed_out = true;
                        }
                    }
                }
            }
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
                    epoch: _,
                    started: _,
                    timed_out,
                } => {
                    if *timed_out
                        || players
                            .read()
                            .keys()
                            .all(|uid| drawing == uid || correct_scores.contains_key(uid))
                    {
                        if *timed_out {
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

fn guesser_score(elapsed: time::Duration) -> u32 {
    (GUESS_SECONDS - elapsed.whole_seconds() as u32) * PERFECT_GUESS_SCORE / GUESS_SECONDS
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
