use crate::api::WsEndpoint;
use crate::domain::{Color, LineWidth, Lobby, Nickname, UserId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Game {
    Heartbeat,
    Players { players: BTreeMap<UserId, Player> },
    Canvas { event: Canvas },
    CanvasBulk { events: Vec<Canvas> },
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GameReq {
    Join { lobby: Lobby, nickname: Nickname },
    Canvas { event: Canvas },
}

impl WsEndpoint for Game {
    const PATH: &'static str = "game";
    type Req = GameReq;
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Player {
    pub nickname: Nickname,
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
}
