use crate::api::TypedWebSocket;
use anyhow::Error;
use ferrogallic_shared::api::{Game, GameReq};
use futures::{SinkExt, StreamExt};

pub async fn game(mut ws: TypedWebSocket<Game>) -> Result<(), Error> {
    let req = match ws.next().await {
        Some(req) => req,
        None => return Ok(()),
    }?;

    let GameReq::Join { lobby, nickname } = req;

    ws.send(&Game::EchoJoin { lobby, nickname }).await?;

    Ok(())
}
