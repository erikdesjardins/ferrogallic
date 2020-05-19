use crate::api::WsEndpoint;
use crate::config::{DEFAULT_GUESS_SECONDS, DEFAULT_ROUNDS};
use crate::domain::{Color, Epoch, Guess, I12Pair, LineWidth, Lobby, Lowercase, Nickname, UserId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Game {
    Canvas(Canvas),
    Guess(Guess),
    Players(Arc<BTreeMap<UserId, Player>>),
    Game(Arc<GameState>),
    Heartbeat,
    CanvasBulk(Vec<Canvas>),
    GuessBulk(Vec<Guess>),
    ClearGuesses,
}

#[test]
fn game_size() {
    assert_eq!(std::mem::size_of::<Game>(), 40);
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GameReq {
    Canvas(Canvas),
    Choose(Lowercase),
    Guess(Lowercase),
    Join(Lobby, Nickname),
    Remove(UserId, Epoch<UserId>),
}

#[test]
fn gamereq_size() {
    assert_eq!(std::mem::size_of::<GameReq>(), 40);
}

impl WsEndpoint for Game {
    const PATH: &'static str = "game";
    type Req = GameReq;
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GameState {
    pub config: GameConfig,
    pub phase: GamePhase,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GameConfig {
    pub rounds: u8,
    pub guess_seconds: u8,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            rounds: DEFAULT_ROUNDS,
            guess_seconds: DEFAULT_GUESS_SECONDS,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum GamePhase {
    WaitingToStart,
    ChoosingWords {
        round: u8,
        choosing: UserId,
        words: Arc<[Lowercase]>,
    },
    Drawing {
        round: u8,
        drawing: UserId,
        correct: BTreeMap<UserId, u32>,
        word: Lowercase,
        epoch: Epoch<GameState>,
        started: OffsetDateTime,
    },
}

impl Default for GamePhase {
    fn default() -> Self {
        Self::WaitingToStart
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Player {
    pub nick: Nickname,
    pub epoch: Epoch<UserId>,
    pub status: PlayerStatus,
    pub score: u32,
}

impl Player {
    pub fn rankings<'a>(
        players: impl IntoIterator<Item = (&'a UserId, &'a Player)>,
    ) -> impl Iterator<Item = (u64, UserId, &'a Player)> {
        let mut players_by_score = players
            .into_iter()
            .map(|(uid, player)| (*uid, player))
            .collect::<Vec<_>>();
        players_by_score.sort_by_key(|(_, player)| player.score);
        players_by_score.into_iter().rev().enumerate().scan(
            (u32::MAX, 0),
            |(prev_score, prev_rank), (index, (uid, player))| {
                if player.score == *prev_score {
                    Some((*prev_rank, uid, player))
                } else {
                    let rank = index as u64 + 1;
                    *prev_score = player.score;
                    *prev_rank = rank;
                    Some((rank, uid, player))
                }
            },
        )
    }
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum PlayerStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub enum Canvas {
    Line {
        from: I12Pair,
        to: I12Pair,
        width: LineWidth,
        color: Color,
    },
    Fill {
        at: I12Pair,
        color: Color,
    },
    PushUndo,
    PopUndo,
    Clear,
}

#[test]
fn canvas_size() {
    assert_eq!(std::mem::size_of::<Canvas>(), 12);
}
