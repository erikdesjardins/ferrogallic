use anyhow::Error;
use serde_json;
use thiserror::Error;
use yew::services::fetch::{FetchService, FetchTask, Request, Response, StatusCode};
use yew::{Component, ComponentLink};

use ferrogallic_api::ApiEndpoint;

pub trait FetchServiceExt {
    fn fetch_api<C: Component, T: ApiEndpoint>(
        &mut self,
        link: &ComponentLink<C>,
        req: <T as ApiEndpoint>::Req,
        f: impl Fn(Result<T, Error>) -> <C as Component>::Message + 'static,
    ) -> Result<FetchTask, Error>;
}

impl FetchServiceExt for FetchService {
    fn fetch_api<C: Component, T: ApiEndpoint>(
        &mut self,
        link: &ComponentLink<C>,
        req: <T as ApiEndpoint>::Req,
        f: impl Fn(Result<T, Error>) -> <C as Component>::Message + 'static,
    ) -> Result<FetchTask, Error> {
        let body = serde_json::to_vec(&req)?;
        let request = Request::post(T::PATH).body(Ok(body))?;
        self.fetch_binary(
            request,
            link.callback(move |response: Response<Result<Vec<u8>, Error>>| {
                let (meta, data) = response.into_parts();
                match data {
                    Ok(data) if meta.status.is_success() => {
                        let res = serde_json::from_slice(&data);
                        f(res.map_err(Into::into))
                    }
                    Ok(data) => {
                        let lossy_data = String::from_utf8_lossy(&data).to_string();
                        f(Err(BadStatusCode(meta.status, lossy_data).into()))
                    }
                    Err(e) => f(Err(e)),
                }
            }),
        )
    }
}

#[derive(Debug, Error)]
#[error("Status {0}: {1}")]
pub struct BadStatusCode(StatusCode, String);
