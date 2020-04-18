use anyhow::Error;
use ferrogallic_shared::{ApiEndpoint, WsEndpoint};
use std::marker::PhantomData;
use thiserror::Error;
use web_sys::window;
use yew::format::Json;
use yew::services::fetch::{FetchService, FetchTask, Request, Response, StatusCode};
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::{Component, ComponentLink};

pub trait FetchServiceExt {
    fn fetch_api<C: Component, T: ApiEndpoint>(
        &mut self,
        link: &ComponentLink<C>,
        req: &<T as ApiEndpoint>::Req,
        f: impl Fn(Result<T, Error>) -> <C as Component>::Message + 'static,
    ) -> Result<FetchTask, Error>;
}

impl FetchServiceExt for FetchService {
    fn fetch_api<C: Component, T: ApiEndpoint>(
        &mut self,
        link: &ComponentLink<C>,
        req: &<T as ApiEndpoint>::Req,
        f: impl Fn(Result<T, Error>) -> <C as Component>::Message + 'static,
    ) -> Result<FetchTask, Error> {
        let request = Request::post(T::PATH).body(Json(req))?;
        self.fetch_binary(
            request,
            link.callback(move |response: Response<Json<Result<T, Error>>>| {
                let (_, Json(data)) = response.into_parts();
                f(data)
            }),
        )
    }
}

#[derive(Debug, Error)]
#[error("Status {0}")]
pub struct BadStatusCode(StatusCode);

pub trait WebSocketServiceExt {
    fn connect_api<C: Component, T: WsEndpoint>(
        &mut self,
        link: &ComponentLink<C>,
        f: impl Fn(Result<T, Error>) -> <C as Component>::Message + 'static,
        on_notification: impl Fn(WebSocketStatus) -> <C as Component>::Message + 'static,
    ) -> Result<WebSocketApiTask<T>, &str>;
}

impl WebSocketServiceExt for WebSocketService {
    fn connect_api<C: Component, T: WsEndpoint>(
        &mut self,
        link: &ComponentLink<C>,
        f: impl Fn(Result<T, Error>) -> <C as Component>::Message + 'static,
        on_notification: impl Fn(WebSocketStatus) -> <C as Component>::Message + 'static,
    ) -> Result<WebSocketApiTask<T>, &str> {
        let url = match window()
            .map(|w| w.location())
            .and_then(|l| Some((l.protocol().ok()?, l.host().ok()?)))
        {
            Some((proto, host)) => {
                let proto = if proto == "http:" { "ws:" } else { "wss:" };
                format!("{}//{}/{}", proto, host, T::PATH)
            }
            None => {
                return Err("failed to get window.location");
            }
        };
        let task = self.connect(
            &url,
            link.callback(move |Json(res)| f(res)),
            link.callback(on_notification),
        )?;
        Ok(WebSocketApiTask(task, PhantomData))
    }
}

pub struct WebSocketApiTask<T: WsEndpoint>(WebSocketTask, PhantomData<fn(T)>);

impl<T: WsEndpoint> WebSocketApiTask<T> {
    pub fn send_api(&mut self, req: &<T as WsEndpoint>::Req) {
        self.0.send_binary(Json(req))
    }
}
