#![allow(
    clippy::redundant_pattern_matching,
    clippy::single_match,
    clippy::too_many_arguments,
    clippy::vec_init_then_push
)]

use std::env;

mod api;
mod files;
mod opt;
mod reply;
mod server;
mod words;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let opt::Options {
        verbose,
        listen_addr,
    } = argh::from_env();

    let listen_addr = match listen_addr {
        Some(addr) => addr,
        None => {
            let port = env::var("PORT")
                .unwrap_or_else(|e| panic!("No addr in args or PORT env var: {}", e));
            let port = port
                .parse()
                .unwrap_or_else(|e| panic!("Failed to parse PORT env var: {}", e));
            ([0, 0, 0, 0], port).into()
        }
    };

    env_logger::Builder::new()
        .filter_level(match verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        })
        .init();

    server::run(listen_addr).await;
}
