use std::net::SocketAddr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about)]
pub struct Options {
    /// Logging verbosity (-v info, -vv debug, -vvv trace)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences), global = true)]
    pub verbose: u8,

    pub listen_addr: Option<SocketAddr>,
}
