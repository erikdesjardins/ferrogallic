#![allow(
    clippy::redundant_pattern_matching,
    clippy::single_match,
    clippy::too_many_arguments,
    clippy::vec_init_then_push
)]

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
    } = clap::Parser::parse();

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
