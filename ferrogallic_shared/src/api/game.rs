use crate::api::WsEndpoint;
use crate::domain::{Color, Epoch, Guess, LineWidth, Lobby, Lowercase, Nickname, U12Pair, UserId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Game {
    Canvas(Canvas),
    Guess(Guess),
    Players(Arc<BTreeMap<UserId, Player>>),
    Game(Arc<GameState>),
    Heartbeat,
    CanvasBulk(Vec<Canvas>),
    GuessBulk(Vec<Guess>),
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
    Remove(UserId, Epoch),
}

#[test]
fn gamereq_size() {
    assert_eq!(std::mem::size_of::<GameReq>(), 40);
}

impl WsEndpoint for Game {
    const PATH: &'static str = "game";
    type Req = GameReq;
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum GameState {
    WaitingToStart {
        starting: bool,
    },
    ChoosingWords {
        choosing: UserId,
        words: Arc<[Lowercase]>,
    },
    Drawing {
        drawing: UserId,
        correct_scores: BTreeMap<UserId, u32>,
        word: Lowercase,
        seconds_remaining: u8,
    },
}

impl Default for GameState {
    fn default() -> Self {
        Self::WaitingToStart { starting: false }
    }
}

#[test]
fn gamestate_size() {
    assert_eq!(std::mem::size_of::<GameState>(), 56);
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Player {
    pub nick: Nickname,
    pub epoch: Epoch,
    pub status: PlayerStatus,
    pub score: u32,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum PlayerStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub enum Canvas {
    Line {
        from: U12Pair,
        to: U12Pair,
        width: LineWidth,
        color: Color,
    },
    Fill {
        at: U12Pair,
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
