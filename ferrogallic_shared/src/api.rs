use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod game;
pub mod lobby;

pub trait ApiEndpoint: Serialize + DeserializeOwned + Send + 'static {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned + Send;
}

pub trait WsEndpoint: Serialize + DeserializeOwned + Send + 'static {
    const PATH: &'static str;
    type Req: Serialize + DeserializeOwned + Send;
}
