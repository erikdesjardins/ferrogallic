use std::fmt::Display;
use std::net::SocketAddr;

use warp::http::StatusCode;
use warp::reply::{Json, WithStatus};
use warp::{Filter, Rejection};

use ferrogallic_api::ApiEndpoint;

use crate::api;
use crate::reply::{bytes, string};
use crate::web_files;

pub async fn run(addr: SocketAddr) {
    let static_files = warp::get().and(warp::path("static")).and({
        let main_js = warp::path!("main.js").map(|| bytes(web_files::JS, "application/javascript"));
        let main_wasm = warp::path!("main.wasm").map(|| bytes(web_files::WASM, "application/wasm"));
        let index_js = warp::path!("index.js").map(|| {
            string(
                "import init from '/static/main.js'; init('/static/main.wasm');",
                "application/javascript",
            )
        });
        main_js.or(main_wasm).or(index_js)
    });

    let index = warp::get().map(|| {
        string(
            "<!doctype html><html><body><script type=module src='/static/index.js'></script></body></html>",
            "text/html",
        )
    });

    let api = warp::post()
        .and(warp::body::content_length_limit(4 * 1024))
        .and({
            let random_lobby_name = endpoint(api::lobby::random_name);
            random_lobby_name
        });

    let server = static_files
        .or(api)
        .or(index)
        .with(warp::log(env!("CARGO_PKG_NAME")));

    warp::serve(server).run(addr).await;
}

fn endpoint<T, E, F>(
    f: F,
) -> impl Filter<Extract = (WithStatus<Json>,), Error = Rejection> + Clone + Send
where
    T: ApiEndpoint,
    E: Display,
    F: Fn(<T as ApiEndpoint>::Req) -> Result<T, E> + Clone + Send,
{
    warp::path(T::PATH)
        .and(warp::body::json())
        .map(f)
        .map(|reply: Result<T, E>| match reply {
            Ok(body) => warp::reply::with_status(warp::reply::json(&body), StatusCode::OK),
            Err(e) => warp::reply::with_status(
                warp::reply::json(&e.to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        })
}
