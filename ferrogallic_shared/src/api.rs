use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod game;
pub mod lobby;

pub trait ApiEndpoint: Serialize + DeserializeOwned + 'static {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned;
}

pub trait WsEndpoint: Serialize + DeserializeOwned + 'static {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned;
}
