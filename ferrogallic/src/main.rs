use structopt::StructOpt;

mod api;
mod opt;
mod reply;
mod server;
mod web_files;
mod words;

#[tokio::main]
async fn main() {
    let opt::Options {
        verbose,
        listen_addr,
    } = opt::Options::from_args();

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
