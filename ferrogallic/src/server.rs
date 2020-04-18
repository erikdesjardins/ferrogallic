use crate::api;
use crate::files;
use crate::reply::{bytes, string};
use std::net::SocketAddr;
use warp::Filter;

pub async fn run(addr: SocketAddr) {
    let favicon = warp::get()
        .and(warp::path!("favicon.ico"))
        .map(|| bytes(files::FAVICON, "image/x-icon"));

    let static_files = warp::get().and(warp::path("static")).and({
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
        main_css.or(main_js).or(main_wasm).or(index_js)
    });

    let index = warp::get().map(|| {
        string(
            concat!(
                "<!doctype html>",
                "<html>",
                "<head><link rel=stylesheet href='/static/main.css'/></head>",
                "<body><script type=module src='/static/index.js'></script></body>",
                "</html>",
            ),
            "text/html",
        )
    });

    let api = warp::post()
        .and(warp::body::content_length_limit(4 * 1024))
        .and({
            let random_lobby_name = api::endpoint(api::lobby::random_name);
            random_lobby_name
        });

    let ws = {
        let game = api::websocket(api::game::game);
        game
    };

    let server = favicon
        .or(static_files)
        .or(api)
        .or(ws)
        .or(index)
        .with(warp::log(env!("CARGO_PKG_NAME")));

    warp::serve(server).run(addr).await;
}
