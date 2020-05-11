pub const MAX_REQUEST_BYTES: u64 = 4 * 1024;
pub const MAX_WS_MESSAGE_BYTES: usize = 4 * 1024;

pub const WS_RX_BUFFER_SHARED: usize = 64;
pub const WS_TX_BUFFER_BROADCAST: usize = 64;

pub const CANVAS_WIDTH: usize = 800;
pub const CANVAS_HEIGHT: usize = 600;

pub const TIMER_TICK_SECONDS: u8 = 10;

pub const NUMBER_OF_WORDS_TO_CHOOSE: usize = 3;
pub const GUESS_SECONDS: u8 = 120;
pub const NOTIFY_TIME_REMAINING_AT: &[u8] = &[60, 30, 10];
pub const PERFECT_GUESS_SCORE: u32 = 500;
pub const CLOSE_GUESS_LEVENSHTEIN: usize = 2;
