use crate::api::WsEndpoint;
use crate::domain::{Color, LineWidth, Lobby, Nickname, UserId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Game {
    Players { players: BTreeMap<UserId, Player> },
    Canvas { event: Canvas },
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
    pub score: u32,
    pub status: PlayerStatus,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum PlayerStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub enum Canvas {
    LineStart {
        x: u16,
        y: u16,
        width: LineWidth,
        color: Color,
    },
    LineTo {
        x: u16,
        y: u16,
    },
    Fill {
        x: u16,
        y: u16,
        color: Color,
    },
}