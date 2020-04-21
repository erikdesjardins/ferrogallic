use crate::api::ApiEndpoint;
use crate::domain::Lobby;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RandomLobbyName {
    pub lobby: Lobby,
}

impl ApiEndpoint for RandomLobbyName {
    const PATH: &'static str = "random_lobby_name";
    type Req = ();
}
