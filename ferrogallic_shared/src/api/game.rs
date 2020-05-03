use crate::api::WsEndpoint;
use crate::domain::{Color, Guess, LineWidth, Lobby, Nickname, UserId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Game {
    Heartbeat,
    Players {
        players: Arc<BTreeMap<UserId, Player>>,
    },
    Game {
        state: Arc<GameState>,
    },
    Canvas {
        event: Canvas,
    },
    CanvasBulk {
        events: Vec<Canvas>,
    },
    Guess {
        guess: Guess,
    },
    GuessBulk {
        guesses: Vec<Guess>,
    },
}

#[test]
fn game_size() {
    assert_eq!(std::mem::size_of::<Game>(), 40);
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GameReq {
    Join { lobby: Lobby, nick: Nickname },
    Choose { word: Box<str> },
    Canvas { event: Canvas },
    Guess { guess: Box<str> },
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
        words: Box<[Box<str>]>,
    },
    Drawing {
        drawing: UserId,
        correct: Vec<UserId>,
        word: Box<str>,
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
        from: (u16, u16),
        to: (u16, u16),
        width: LineWidth,
        color: Color,
    },
    Fill {
        at: (u16, u16),
        color: Color,
    },
    PushUndo,
    PopUndo,
    Clear,
}
