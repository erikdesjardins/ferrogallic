use clap::{ArgAction, Parser};
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Options {
    /// logging verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    pub verbose: u8,

    pub listen_addr: SocketAddr,
}
