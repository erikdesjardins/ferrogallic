use std::net::SocketAddr;

use warp::Filter;

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
            "<!DOCTYPE html><html><body><script type=module src='/static/index.js'></script></body></html>",
            "text/html",
        )
    });

    let server = static_files
        .or(index)
        .with(warp::log(env!("CARGO_PKG_NAME")));

    warp::serve(server).run(addr).await;
}
