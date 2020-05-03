use std::time::Duration;

pub const MAX_REQUEST_BYTES: u64 = 4 * 1024;
pub const MAX_WS_MESSAGE_BYTES: usize = 4 * 1024;

pub const WS_RX_BUFFER_SHARED: usize = 64;
pub const WS_TX_BUFFER_BROADCAST: usize = 64;

pub const REMOVE_DISCONNECTED_PLAYERS: Duration = Duration::from_secs(60);

pub const CANVAS_WIDTH: usize = 800;
pub const CANVAS_HEIGHT: usize = 600;

pub const NUMBER_OF_WORDS_TO_CHOOSE: usize = 3;
pub const GUESS_SECONDS: u8 = 120;
pub const PERFECT_GUESS_SCORE: u32 = 500;
