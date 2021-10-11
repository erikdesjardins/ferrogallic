use argh::FromArgs;
use std::net::SocketAddr;

/// Clone of skribble.io.
#[derive(Debug, FromArgs)]
pub struct Options {
    /// logging verbosity (-v info, -v -v debug, -v -v -v trace)
    #[argh(switch, short = 'v')]
    pub verbose: u8,

    #[argh(positional)]
    pub listen_addr: SocketAddr,
}
