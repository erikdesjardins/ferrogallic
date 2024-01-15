use anyhow::{anyhow, Error};
use ferrogallic_shared::api::{ApiEndpoint, WsEndpoint};
use ferrogallic_shared::paths;
use futures::stream::{SplitSink, SplitStream};
use futures::{FutureExt, SinkExt, StreamExt};
use gloo::net::http::Request;
use gloo::net::websocket::futures::WebSocket;
use gloo::net::websocket::Message;
use js_sys::Uint8Array;
use std::marker::PhantomData;
use web_sys::window;

pub async fn fetch_api<T: ApiEndpoint>(req: &T::Req) -> Result<T, Error> {
    let url = format!("/{}/{}", paths::api::PREFIX, T::PATH);
    let payload = bincode::serialize(req)?;
    let body = {
        // Safety: the vec backing `payload` is not resized, modified, or dropped while the Uint8Array exists
        let payload = unsafe { Uint8Array::view(&payload) };
        let response = Request::post(&url).body(payload)?.send().await?;
        response.binary().await?
    };
    drop(payload);
    let parsed = bincode::deserialize(&body)?;
    Ok(parsed)
}

pub fn connect_api<T: WsEndpoint>(
) -> Result<(TypedWebSocketReader<T>, TypedWebSocketWriter<T>), Error> {
    let url = match window()
        .map(|w| w.location())
        .and_then(|l| Some((l.protocol().ok()?, l.host().ok()?)))
    {
        Some((proto, host)) => {
            let proto = if proto == "http:" { "ws:" } else { "wss:" };
            format!("{}//{}/{}/{}", proto, host, paths::ws::PREFIX, T::PATH)
        }
        None => {
            return Err(anyhow!("Failed to get window.location"));
        }
    };
    let socket = WebSocket::open(&url).map_err(|e| anyhow!("Failed to open websocket: {}", e))?;
    let (writer, reader) = socket.split();
    Ok((
        TypedWebSocketReader(reader, PhantomData),
        TypedWebSocketWriter(writer, PhantomData),
    ))
}

pub struct TypedWebSocketReader<T: WsEndpoint>(SplitStream<WebSocket>, PhantomData<fn() -> T>);

impl<T: WsEndpoint> TypedWebSocketReader<T> {
    pub async fn next_api(&mut self) -> Option<Result<T, Error>> {
        self.0.next().await.map(|msg| {
            let msg = msg.map_err(|e| anyhow!("Failed to read websocket message: {}", e))?;
            let body = match msg {
                Message::Bytes(v) => v,
                Message::Text(s) => return Err(anyhow!("Unexpected string ws body: {}", s)),
            };
            let parsed = bincode::deserialize(&body)?;
            Ok(parsed)
        })
    }
}

pub struct TypedWebSocketWriter<T: WsEndpoint>(
    SplitSink<WebSocket, Message>,
    PhantomData<fn() -> T>,
);

impl<T: WsEndpoint> TypedWebSocketWriter<T> {
    pub async fn wait_for_connection_and_send(&mut self, req: &T::Req) -> Result<(), Error> {
        let payload = bincode::serialize(req)?;
        self.0
            .send(Message::Bytes(payload))
            .await
            .map_err(|e| anyhow!("Failed to send websocket message: {}", e))?;
        Ok(())
    }

    pub fn send_sync(&mut self, req: &T::Req) -> Result<(), Error> {
        let payload = bincode::serialize(req)?;
        match self.0.send(Message::Bytes(payload)).now_or_never() {
            Some(r) => r.map_err(|e| anyhow!("Failed to send websocket message: {}", e)),
            None => Err(anyhow!("Websocket wasn't ready to send")),
        }
    }
}
