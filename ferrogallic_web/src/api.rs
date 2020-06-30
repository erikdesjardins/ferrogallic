use anyhow::{anyhow, Error};
use ferrogallic_shared::api::{ApiEndpoint, WsEndpoint};
use std::marker::PhantomData;
use thiserror::Error;
use web_sys::window;
use yew::format::Bincode;
use yew::services::fetch::{FetchService, FetchTask, Request, Response, StatusCode};
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::{Component, ComponentLink};

pub struct FetchServiceExt(());

impl FetchServiceExt {
    pub fn fetch_api<C: Component, T: ApiEndpoint>(
        link: &ComponentLink<C>,
        req: &T::Req,
        f: impl Fn(Result<T, Error>) -> C::Message + 'static,
    ) -> Result<FetchTask, Error> {
        let request = Request::post(format!("/{}", T::PATH)).body(Bincode(req))?;
        FetchService::fetch_binary(
            request,
            link.callback(move |response: Response<Bincode<Result<T, Error>>>| {
                let (_, Bincode(data)) = response.into_parts();
                f(data)
            }),
        )
    }
}

#[derive(Debug, Error)]
#[error("Status {0}")]
pub struct BadStatusCode(StatusCode);

pub struct WebSocketServiceExt(());

impl WebSocketServiceExt {
    pub fn connect_api<C: Component, T: WsEndpoint>(
        link: &ComponentLink<C>,
        f: impl Fn(Result<T, Error>) -> C::Message + 'static,
        on_notification: impl Fn(WebSocketStatus) -> C::Message + 'static,
    ) -> Result<WebSocketApiTask<T>, Error> {
        let url = match window()
            .map(|w| w.location())
            .and_then(|l| Some((l.protocol().ok()?, l.host().ok()?)))
        {
            Some((proto, host)) => {
                let proto = if proto == "http:" { "ws:" } else { "wss:" };
                format!("{}//{}/{}", proto, host, T::PATH)
            }
            None => {
                return Err(anyhow!("Failed to get window.location"));
            }
        };
        let task = WebSocketService::connect_binary(
            &url,
            link.callback(move |Bincode(res)| f(res)),
            link.callback(on_notification),
        )
        .map_err(|e| anyhow!(e.to_string()))?;
        Ok(WebSocketApiTask(task, PhantomData))
    }
}

pub struct WebSocketApiTask<T: WsEndpoint>(WebSocketTask, PhantomData<fn(T)>);

impl<T: WsEndpoint> WebSocketApiTask<T> {
    pub fn send_api(&mut self, req: &T::Req) {
        self.0.send_binary(Bincode(req))
    }
}
