pub const MAX_REQUEST_BYTES: u64 = 4 * 1024;
pub const MAX_WS_MESSAGE_BYTES: usize = 4 * 1024;

pub const RX_SHARED_BUFFER: usize = 64;
pub const TX_BROADCAST_BUFFER: usize = 256;
pub const TX_SELF_DELAYED_BUFFER: usize = 4;

pub const CANVAS_WIDTH: usize = 800;
pub const CANVAS_HEIGHT: usize = 600;

pub const HEARTBEAT_SECONDS: u64 = 45;

pub const NUMBER_OF_WORDS_TO_CHOOSE: usize = 3;
pub const DEFAULT_ROUNDS: u8 = 3;
pub const DEFAULT_GUESS_SECONDS: u8 = 120;
pub const PERFECT_GUESS_SCORE: u32 = 500;
pub const CLOSE_GUESS_LEVENSHTEIN: usize = 2;
