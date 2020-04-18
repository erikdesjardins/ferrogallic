use anyhow::Error;
use bincode;
use bytes::buf::BufExt;
use ferrogallic_shared::api::{ApiEndpoint, WsEndpoint};
use ferrogallic_shared::config::MAX_WS_MESSAGE_SIZE;
use futures::ready;
use futures::task::{Context, Poll};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use warp::http::{Response, StatusCode};
use warp::ws::{Message, WebSocket};
use warp::{Filter, Rejection, Reply, Sink, Stream};

pub mod game;
pub mod lobby;

pub fn endpoint<T, E, F>(f: F) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ApiEndpoint,
    E: Into<Error>,
    F: Fn(<T as ApiEndpoint>::Req) -> Result<T, E> + Clone + Send,
{
    warp::path(T::PATH)
        .and(warp::path::end())
        .and(warp::body::aggregate())
        .map(move |buf| {
            let req = match bincode::deserialize_from(BufExt::reader(buf)) {
                Ok(req) => req,
                Err(e) => {
                    log::warn!("Failed to deserialize request '{}': {}", T::PATH, e);
                    return warp::reply::with_status(Response::default(), StatusCode::BAD_REQUEST);
                }
            };
            let reply = match f(req) {
                Ok(reply) => reply,
                Err(e) => {
                    log::error!("Error in API handler '{}': {}", T::PATH, e.into());
                    return warp::reply::with_status(Response::default(), StatusCode::CONFLICT);
                }
            };
            match bincode::serialize(&reply) {
                Ok(body) => warp::reply::with_status(Response::new(body), StatusCode::OK),
                Err(e) => {
                    log::error!("Failed to serialize response '{}': {}", T::PATH, e);
                    return warp::reply::with_status(
                        Response::default(),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    );
                }
            }
        })
}

pub fn websocket<T, F, Fut, E>(
    f: F,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: WsEndpoint,
    F: Fn(TypedWebSocket<T>) -> Fut + Copy + Send + 'static,
    Fut: Future<Output = Result<(), E>> + Send,
    E: Into<Error>,
{
    warp::path(T::PATH)
        .and(warp::path::end())
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            ws.max_message_size(MAX_WS_MESSAGE_SIZE)
                .on_upgrade(move |websocket| async move {
                    let fut = f(TypedWebSocket::new(websocket));
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

pub struct TypedWebSocket<T: WsEndpoint>(WebSocket, PhantomData<fn(T)>);

impl<T: WsEndpoint> TypedWebSocket<T> {
    fn new(ws: WebSocket) -> Self {
        Self(ws, PhantomData)
    }
}

impl<T: WsEndpoint> Stream for TypedWebSocket<T> {
    type Item = Result<<T as WsEndpoint>::Req, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(match ready!(Pin::new(&mut self.0).poll_next(cx)) {
            Some(Ok(msg)) => match bincode::deserialize(msg.as_bytes()) {
                Ok(req) => Some(Ok(req)),
                Err(e) => Some(Err(e.into())),
            },
            Some(Err(e)) => Some(Err(e.into())),
            None => None,
        })
    }
}

impl<T: WsEndpoint> Sink<&T> for TypedWebSocket<T> {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(match ready!(Pin::new(&mut self.0).poll_ready(cx)) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        })
    }

    fn start_send(mut self: Pin<&mut Self>, item: &T) -> Result<(), Self::Error> {
        match bincode::serialize(item) {
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
