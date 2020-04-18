use anyhow::Error;
use futures::{SinkExt, StreamExt};

use ferrogallic_shared::api::{Game, GameReq};

use crate::api::JsonWebSocket;

pub async fn game(mut ws: JsonWebSocket<Game>) -> Result<(), Error> {
    let req = match ws.next().await {
        Some(req) => req,
        None => return Ok(()),
    }?;

    let GameReq::Join { lobby, nickname } = req;

    ws.send(&Game::EchoJoin { lobby, nickname }).await?;

    Ok(())
}
