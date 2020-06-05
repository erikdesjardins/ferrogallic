use crate::api;
use crate::files;
use crate::reply::{bytes, string};
use ferrogallic_shared::config::MAX_REQUEST_BYTES;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::Filter;

#[allow(clippy::let_and_return)]
pub async fn run(addr: SocketAddr) {
    let static_files = warp::get().and(warp::path("static")).and({
        let favicon = warp::path!("favicon.png").map(|| bytes(files::FAVICON, "image/png"));
        let main_css = warp::path!("main.css").map(|| bytes(files::web::CSS, "text/css"));
        let main_js =
            warp::path!("main.js").map(|| bytes(files::web::JS, "application/javascript"));
        let main_wasm =
            warp::path!("main.wasm").map(|| bytes(files::web::WASM, "application/wasm"));
        let index_js = warp::path!("index.js").map(|| {
            string(
                "import init from '/static/main.js'; init('/static/main.wasm');",
                "application/javascript",
            )
        });
        favicon.or(main_css).or(main_js).or(main_wasm).or(index_js)
    });

    let index = warp::get().map(|| {
        string(
            concat!(
                "<!doctype html>",
                "<html>",
                "<head>",
                "<meta name=robots content='noindex, nofollow'/>",
                "<meta name=viewport content='width=1200, initial-scale=0.5, maximum-scale=1'>",
                "<link rel=icon href='/static/favicon.png'/>",
                "<link rel=stylesheet href='/static/main.css'/>",
                "</head>",
                "<body><script type=module src='/static/index.js'></script></body>",
                "</html>",
            ),
            "text/html",
        )
    });

    let state = Arc::default();

    let api = warp::post()
        .and(warp::body::content_length_limit(MAX_REQUEST_BYTES))
        .and({
            let random_lobby_name = api::endpoint((), api::lobby::random_name);
            random_lobby_name
        });

    let ws = {
        let game = api::websocket(state, api::game::join_game);
        game
    };

    let server = static_files
        .or(api)
        .or(ws)
        .or(index)
        .with(warp::log(env!("CARGO_PKG_NAME")));

    warp::serve(server).run(addr).await;
}
