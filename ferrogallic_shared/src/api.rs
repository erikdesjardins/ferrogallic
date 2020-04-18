use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub trait ApiEndpoint: Serialize + DeserializeOwned + Send + 'static {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned + Send;
}

#[derive(Deserialize, Serialize)]
pub struct RandomLobbyName {
    pub lobby: String,
}

impl ApiEndpoint for RandomLobbyName {
    const PATH: &'static str = "random_lobby_name";
    type Req = ();
}

pub trait WsEndpoint: Serialize + DeserializeOwned + Send + 'static {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned + Send;
}

#[derive(Deserialize, Serialize)]
pub enum Game {
    EchoJoin { lobby: String, nickname: String },
}

#[derive(Deserialize, Serialize)]
pub enum GameReq {
    Join { lobby: String, nickname: String },
}

impl WsEndpoint for Game {
    const PATH: &'static str = "game";
    type Req = GameReq;
}
