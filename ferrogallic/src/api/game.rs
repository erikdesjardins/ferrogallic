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
use std::mem;
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
                let player = match players.read().get(&user_id) {
                    Some(player) if player.epoch == epoch => player,
                    _ => {
                        tx.send(Broadcast::Kill(user_id, epoch))?;
                        continue;
                    }
                };
                let state = game_state.read().as_ref();
                match (req, state) {
                    (GameReq::Canvas(event), _) => {
                        (&tx, &mut canvas_events).send(user_id, event)?;
                        continue;
                    }
                    (GameReq::Choose(word), GameState::ChoosingWords { choosing, words })
                        if *choosing == user_id && words.contains(&word) =>
                    {
                        let drawing = *choosing;
                        trans_to_drawing(
                            &tx,
                            &mut tx_self_delayed,
                            Arc::make_mut(game_state.write()),
                            &mut canvas_events,
                            &mut guesses,
                            drawing,
                            word,
                        )
                        .await?;
                    }
                    (GameReq::Guess(guess), state) => match state {
                        GameState::WaitingToStart => match guess.as_ref() {
                            "start" => trans_to_choosing(
                                &tx,
                                players.read(),
                                Arc::make_mut(game_state.write()),
                                &mut guesses,
                                None,
                            )?,
                            _ => (&tx, &mut guesses).send(Guess::Message(user_id, guess))?,
                        },
                        GameState::ChoosingWords { .. } => {
                            (&tx, &mut guesses).send(Guess::Message(user_id, guess))?;
                        }
                        GameState::Drawing {
                            drawing,
                            correct,
                            word,
                            epoch: _,
                            started,
                        } => {
                            if *drawing == user_id || correct.contains_key(&user_id) {
                                (&tx, &mut guesses).send(Guess::Message(user_id, guess))?;
                            } else if guess == *word {
                                let elapsed = OffsetDateTime::now_utc() - *started;
                                if let GameState::Drawing { correct, .. } =
                                    Arc::make_mut(game_state.write())
                                {
                                    correct.insert(user_id, guesser_score(elapsed));
                                }
                                (&tx, &mut guesses).send(Guess::Correct(user_id))?;
                            } else {
                                if levenshtein(&guess, word) <= CLOSE_GUESS_LEVENSHTEIN {
                                    tx.send(Broadcast::Only(
                                        user_id,
                                        Game::Guess(Guess::CloseGuess(guess.clone())),
                                    ))?;
                                }
                                (&tx, &mut guesses).send(Guess::Guess(user_id, guess))?;
                            }
                        }
                    },
                    (GameReq::Remove(remove_uid, remove_epoch), _) => {
                        if let Entry::Occupied(entry) =
                            Arc::make_mut(players.write()).entry(remove_uid)
                        {
                            if entry.get().epoch == remove_epoch {
                                let removed = entry.remove();
                                log::info!("Lobby={} Player={} removed", lobby, removed.nick);
                            }
                        }
                    }
                    (req @ GameReq::Choose(..), _) | (req @ GameReq::Join(..), _) => {
                        log::warn!("Lobby={} Player={} invalid: {:?}", lobby, player.nick, req);
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
                        let game_state = Arc::make_mut(game_state.write());
                        if let GameState::Drawing {
                            drawing,
                            correct,
                            word,
                            ..
                        } = game_state
                        {
                            (&tx, &mut guesses).send(Guess::TimeExpired(word.clone()))?;
                            let drawing = *drawing;
                            let correct = mem::take(correct);
                            trans_at_round_end(
                                &tx,
                                Arc::make_mut(players.write()),
                                game_state,
                                &mut guesses,
                                drawing,
                                correct,
                            )?;
                        }
                    }
                }
            }
        }

        if players.is_changed() || game_state.is_changed() {
            if let GameState::Drawing {
                drawing, correct, ..
            } = game_state.read().as_ref()
            {
                if players
                    .read()
                    .keys()
                    .all(|uid| drawing == uid || correct.contains_key(uid))
                {
                    let game_state = Arc::make_mut(game_state.write());
                    if let GameState::Drawing {
                        drawing, correct, ..
                    } = game_state
                    {
                        let drawing = *drawing;
                        let correct = mem::take(correct);
                        trans_at_round_end(
                            &tx,
                            Arc::make_mut(players.write()),
                            game_state,
                            &mut guesses,
                            drawing,
                            correct,
                        )?;
                    }
                }
            }
        }

        if let Some(players) = players.reset_if_changed() {
            tx.send(Broadcast::Everyone(Game::Players(players.clone())))?;
        }
        if let Some(state) = game_state.reset_if_changed() {
            tx.send(Broadcast::Everyone(Game::Game(state.clone())))?;
        }
    }
}

trait CanvasExt {
    fn send(self, user_id: UserId, event: Canvas) -> Result<(), GameLoopError>;

    fn clear(self) -> Result<(), GameLoopError>;
}

impl CanvasExt for (&broadcast::Sender<Broadcast>, &mut Vec<Canvas>) {
    fn send(self, user_id: UserId, event: Canvas) -> Result<(), GameLoopError> {
        self.1.push(event);
        self.0
            .send(Broadcast::Exclude(user_id, Game::Canvas(event)))?;
        Ok(())
    }

    fn clear(self) -> Result<(), GameLoopError> {
        self.1.clear();
        self.0
            .send(Broadcast::Everyone(Game::Canvas(Canvas::Clear)))?;
        Ok(())
    }
}

trait GuessExt {
    fn send(self, guess: Guess) -> Result<(), GameLoopError>;
}

impl GuessExt for (&broadcast::Sender<Broadcast>, &mut Vec<Guess>) {
    fn send(self, guess: Guess) -> Result<(), GameLoopError> {
        self.1.push(guess.clone());
        self.0.send(Broadcast::Everyone(Game::Guess(guess)))?;
        Ok(())
    }
}

fn trans_to_choosing(
    tx: &broadcast::Sender<Broadcast>,
    players: &BTreeMap<UserId, Player>,
    game_state: &mut GameState,
    guesses: &mut Vec<Guess>,
    previously_drawing: Option<UserId>,
) -> Result<(), GameLoopError> {
    let choosing = players
        .keys()
        // first player after the previous drawer...
        .skip_while(|uid| Some(**uid) != previously_drawing)
        .nth(1)
        // ...or the first player in the list
        .or_else(|| players.keys().next());
    let choosing = match choosing {
        Some(choosing) => *choosing,
        None => return Err(GameLoopError::NoConnectionsDuringStateChange),
    };
    let words = words::GAME
        .choose_multiple(&mut thread_rng(), NUMBER_OF_WORDS_TO_CHOOSE)
        .copied()
        .map(Lowercase::new)
        .collect();
    *game_state = GameState::ChoosingWords { choosing, words };
    (tx, guesses).send(Guess::NowChoosing(choosing))?;
    Ok(())
}

async fn trans_to_drawing(
    tx: &broadcast::Sender<Broadcast>,
    tx_self_delayed: &mut mpsc::Sender<(GameLoop, Instant)>,
    game_state: &mut GameState,
    canvas_events: &mut Vec<Canvas>,
    guesses: &mut Vec<Guess>,
    drawing: UserId,
    word: Lowercase,
) -> Result<(), GameLoopError> {
    let game_epoch = Epoch::next();
    let started = OffsetDateTime::now_utc();
    let will_end = Instant::now() + Duration::from_secs(GUESS_SECONDS.into());
    *game_state = GameState::Drawing {
        drawing,
        correct: Default::default(),
        word,
        epoch: game_epoch,
        started,
    };
    (tx, guesses).send(Guess::NowDrawing(drawing))?;
    (tx, canvas_events).clear()?;
    tx_self_delayed
        .send((GameLoop::GameEnd(game_epoch), will_end))
        .await?;
    Ok(())
}

fn trans_at_round_end(
    tx: &broadcast::Sender<Broadcast>,
    players: &mut BTreeMap<UserId, Player>,
    game_state: &mut GameState,
    guesses: &mut Vec<Guess>,
    drawing: UserId,
    correct: BTreeMap<UserId, u32>,
) -> Result<(), GameLoopError> {
    for (&user_id, &score) in &correct {
        if let Some(player) = players.get_mut(&user_id) {
            player.score += score;
        }
        (tx, &mut *guesses).send(Guess::EarnedPoints(user_id, score))?;
    }
    let drawer_score = drawer_score(correct.values().copied(), players.len() as u32);
    if let Some(drawer) = players.get_mut(&drawing) {
        drawer.score += drawer_score;
    }
    trans_to_choosing(tx, players, game_state, guesses, Some(drawing))?;
    Ok(())
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
