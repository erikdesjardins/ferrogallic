use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub trait ApiEndpoint: Serialize + DeserializeOwned {
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
