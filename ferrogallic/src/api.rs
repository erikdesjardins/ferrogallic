use anyhow::Error;
use ferrogallic_shared::{ApiEndpoint, WsEndpoint};
use futures::ready;
use futures::task::{Context, Poll};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use warp::http::StatusCode;
use warp::reply::{Json, WithStatus};
use warp::ws::{Message, WebSocket};
use warp::{Filter, Rejection, Reply, Sink, Stream};

pub mod game;
pub mod lobby;

pub fn endpoint<T, E, F>(
    f: F,
) -> impl Filter<Extract = (WithStatus<Json>,), Error = Rejection> + Clone
where
    T: ApiEndpoint,
    E: Into<Error>,
    F: Fn(<T as ApiEndpoint>::Req) -> Result<T, E> + Clone + Send,
{
    warp::path(T::PATH)
        .and(warp::path::end())
        .and(warp::body::json())
        .map(f)
        .map(|reply: Result<T, E>| match reply {
            Ok(body) => warp::reply::with_status(warp::reply::json(&body), StatusCode::OK),
            Err(e) => {
                let e = e.into();
                log::error!("Error in API handler '{}': {}", T::PATH, e);
                warp::reply::with_status(
                    warp::reply::json(&e.to_string()),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        })
}

pub fn websocket<T, F, Fut, E>(
    f: F,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: WsEndpoint,
    F: Fn(JsonWebSocket<T>) -> Fut + Copy + Send + 'static,
    Fut: Future<Output = Result<(), E>> + Send,
    E: Into<Error>,
{
    warp::path(T::PATH)
        .and(warp::path::end())
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            ws.max_message_size(4 * 1024 * 1024)
                .on_upgrade(move |websocket| async move {
                    let fut = f(JsonWebSocket(websocket, PhantomData));
                    match fut.await {
                        Ok(()) => (),
                        Err(e) => {
                            log::error!("Error in WS handler '{}': {}", T::PATH, e.into());
                            ()
                        }
                    }
                })
        })
}

pub struct JsonWebSocket<T: WsEndpoint>(WebSocket, PhantomData<fn(T)>);

impl<T: WsEndpoint> Stream for JsonWebSocket<T> {
    type Item = Result<<T as WsEndpoint>::Req, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(match ready!(Pin::new(&mut self.0).poll_next(cx)) {
            Some(Ok(msg)) => match serde_json::from_slice(&msg.into_bytes()) {
                Ok(req) => Some(Ok(req)),
                Err(e) => Some(Err(e.into())),
            },
            Some(Err(e)) => Some(Err(e.into())),
            None => None,
        })
    }
}

impl<T: WsEndpoint> Sink<&T> for JsonWebSocket<T> {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(match ready!(Pin::new(&mut self.0).poll_ready(cx)) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        })
    }

    fn start_send(mut self: Pin<&mut Self>, item: &T) -> Result<(), Self::Error> {
        match serde_json::to_vec(item) {
            Ok(msg) => match Pin::new(&mut self.0).start_send(Message::binary(msg)) {
                Ok(()) => Ok(()),
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(match ready!(Pin::new(&mut self.0).poll_flush(cx)) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        })
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(match ready!(Pin::new(&mut self.0).poll_close(cx)) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        })
    }
}
