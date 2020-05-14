use crate::config::{CANVAS_HEIGHT, CANVAS_WIDTH};
use serde::{Deserialize, Serialize};
use std::alloc;
use std::collections::hash_map::DefaultHasher;
use std::convert::Infallible;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::ptr;
use std::slice;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct UserId(u64);

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Nickname(Arc<str>);

impl Nickname {
    pub fn new(nick: impl Into<Arc<str>>) -> Self {
        Self(nick.into())
    }

    pub fn user_id(&self) -> UserId {
        let mut s = DefaultHasher::new();
        self.0.hash(&mut s);
        UserId(s.finish())
    }
}

impl Deref for Nickname {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for Nickname {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.into()))
    }
}

impl fmt::Display for Nickname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Lobby(Arc<str>);

impl Lobby {
    pub fn new(lobby: impl Into<Arc<str>>) -> Self {
        Self(lobby.into())
    }
}

impl Deref for Lobby {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for Lobby {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.into()))
    }
}

impl fmt::Display for Lobby {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Epoch<T>(NonZeroUsize, PhantomData<T>);

impl<T> Copy for Epoch<T> {}

impl<T> Clone for Epoch<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Epoch<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Epoch<T> {}

impl<T> Epoch<T> {
    pub fn next() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(1);

        let epoch = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(NonZeroUsize::new(epoch).unwrap(), PhantomData)
    }
}

impl<T> fmt::Display for Epoch<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Lowercase(Arc<str>);

impl Lowercase {
    pub fn new(str: impl Into<String>) -> Self {
        let mut str = str.into();
        str.make_ascii_lowercase();
        Self(str.into())
    }
}

impl Default for Lowercase {
    fn default() -> Self {
        Self(Arc::from(""))
    }
}

impl Deref for Lowercase {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Lowercase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Guess {
    System(Arc<str>),
    Message(UserId, Lowercase),
    NowChoosing(UserId),
    NowDrawing(UserId),
    Guess(UserId, Lowercase),
    CloseGuess(Lowercase),
    Correct(UserId),
    EarnedPoints(UserId, u32),
    TimeExpired(Lowercase),
}

#[test]
fn guess_size() {
    assert_eq!(std::mem::size_of::<Guess>(), 32);
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum LineWidth {
    Small,
    Normal,
    Medium,
    Large,
    Extra,
}

impl Default for LineWidth {
    fn default() -> Self {
        Self::Normal
    }
}

impl LineWidth {
    pub fn scanlines(self) -> &'static [u16] {
        match self {
            Self::Small => {
                // 0.5px radius
                // + 1
                &[1]
            }
            Self::Normal => {
                // 1px radius
                // +++
                // +++ 3
                // +++ 3
                &[3, 3]
            }
            Self::Medium => {
                // 2px radius
                //  +++
                // +++++
                // +++++ 5
                // +++++ 5
                //  +++  3
                &[5, 5, 3]
            }
            Self::Large => {
                // 4px radius
                //   +++++
                //  +++++++
                // +++++++++
                // +++++++++
                // +++++++++ 9
                // +++++++++ 9
                // +++++++++ 9
                //  +++++++  7
                //   +++++   5
                &[9, 9, 9, 7, 5]
            }
            Self::Extra => {
                // 7px radius
                &[15, 15, 15, 13, 13, 11, 9, 5]
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Tool {
    Pen(LineWidth),
    Fill,
}

impl Default for Tool {
    fn default() -> Self {
        Self::Pen(Default::default())
    }
}

impl Tool {
    pub const ALL: [Self; 6] = [
        Self::Pen(LineWidth::Small),
        Self::Pen(LineWidth::Normal),
        Self::Pen(LineWidth::Medium),
        Self::Pen(LineWidth::Large),
        Self::Pen(LineWidth::Extra),
        Self::Fill,
    ];
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct I12Pair {
    bytes: [u8; 3],
}

impl I12Pair {
    pub fn new(x: i16, y: i16) -> Self {
        debug_assert!(-(1 << 11) <= x && x < (1 << 11), "x={} out of range", x);
        debug_assert!(-(1 << 11) <= y && y < (1 << 11), "y={} out of range", y);

        Self {
            bytes: [
                x as u8,
                (x >> 8 & 0xf) as u8 | (y << 4) as u8,
                (y >> 4) as u8,
            ],
        }
    }

    pub fn x(self) -> i16 {
        let unsigned = self.bytes[0] as u16 | (self.bytes[1] as u16 & 0xf) << 8;
        // sign-extend
        ((unsigned << 4) as i16) >> 4
    }

    pub fn y(self) -> i16 {
        let unsigned = (self.bytes[1] as u16) >> 4 | (self.bytes[2] as u16) << 4;
        // sign-extend
        ((unsigned << 4) as i16) >> 4
    }
}

#[test]
fn i12pair_exhaustive() {
    for x in -(1 << 11)..(1 << 11) {
        let pair = I12Pair::new(x, 0x7a5);
        assert_eq!(pair.x(), x);
        assert_eq!(pair.y(), 0x7a5);
    }
    for y in -(1 << 11)..(1 << 11) {
        let pair = I12Pair::new(0x7a5, y);
        assert_eq!(pair.x(), 0x7a5);
        assert_eq!(pair.y(), y);
    }
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const WHITE: Self = Self::new(0xff, 0xff, 0xff);
    pub const BLACK: Self = Self::new(0x00, 0x00, 0x00);

    pub const ALL: [Self; 33] = [
        Self::WHITE,                 // White
        Self::BLACK,                 // Black
        Self::new(0x7F, 0x7F, 0x7F), // 50% Grey
        Self::new(0xC1, 0xC1, 0xC1), // Grey
        Self::new(0x4C, 0x4C, 0x4C), // DarkGrey
        Self::new(0xE0, 0xE0, 0xE0), // LightGrey
        Self::new(0xEF, 0x13, 0x0B), // Red
        Self::new(0x74, 0x0B, 0x07), // DarkRed
        Self::new(0xF9, 0x86, 0x82), // LightRed
        Self::new(0xFF, 0x71, 0x00), // Orange
        Self::new(0xC2, 0x38, 0x00), // DarkOrange
        Self::new(0xFF, 0xB8, 0x7F), // LightOrange
        Self::new(0xFF, 0xE4, 0x00), // Yellow
        Self::new(0xE8, 0xA2, 0x00), // DarkYellow
        Self::new(0xFF, 0xF1, 0x7F), // LightYellow
        Self::new(0x00, 0xCC, 0x00), // Green
        Self::new(0x00, 0x55, 0x10), // DarkGreen
        Self::new(0x65, 0xFF, 0x65), // LightGreen
        Self::new(0x00, 0xB2, 0xFF), // Blue
        Self::new(0x00, 0x56, 0x9E), // DarkBlue
        Self::new(0x7F, 0xD8, 0xFF), // LightBlue
        Self::new(0x23, 0x1F, 0xD3), // Indigo
        Self::new(0x0E, 0x08, 0x65), // DarkIndigo
        Self::new(0x8C, 0x8A, 0xED), // LightIndigo
        Self::new(0xA3, 0x00, 0xBA), // Violet
        Self::new(0x55, 0x00, 0x69), // DarkViolet
        Self::new(0xEA, 0x5D, 0xFF), // LightViolet
        Self::new(0xD3, 0x7C, 0xAA), // Pink
        Self::new(0xA7, 0x55, 0x74), // DarkPink
        Self::new(0xE9, 0xBD, 0xD4), // LightPink
        Self::new(0xA0, 0x52, 0x2D), // Brown
        Self::new(0x63, 0x30, 0x0D), // DarkBrown
        Self::new(0xDD, 0xA3, 0x87), // LightBrown
    ];

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { a: 0xff, r, g, b }
    }
}

#[repr(transparent)]
pub struct CanvasBuffer([[Color; CANVAS_WIDTH]; CANVAS_HEIGHT]);

impl CanvasBuffer {
    pub fn boxed() -> Box<Self> {
        // avoid blowing the wasm stack
        let layout = alloc::Layout::new::<Self>();
        // Safety: layout is valid for type, allocator is initialized
        // Safety: type can be safely zero-initialized, as it is repr(transparent) over an array whose elements can safely be zero-initialized
        let ptr = unsafe { alloc::alloc_zeroed(layout) as *mut Self };
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        // Safety: ptr is non-null, unowned, and points to a valid object (since it was zero-initialized)
        unsafe { Box::from_raw(ptr) }
    }

    pub fn clone_boxed(&self) -> Box<Self> {
        let layout = alloc::Layout::new::<Self>();
        // Safety: layout is valid for type, allocator is initialized
        let ptr = unsafe { alloc::alloc(layout) as *mut Self };
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        // Safety: type contains only Copy types, so it can be safely bitwise copied; ptr was just allocated so it cannot overlap
        unsafe { ptr::copy_nonoverlapping(self, ptr, 1) };
        // Safety: ptr is non-null, unowned, and points to a valid object (since it was fully overwritten by a valid object)
        unsafe { Box::from_raw(ptr) }
    }

    pub fn x_len(&self) -> usize {
        self.0[0].len()
    }

    pub fn y_len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, x: usize, y: usize) -> Color {
        self.0
            .get(y)
            .and_then(|row| row.get(x))
            .copied()
            .unwrap_or(Color::TRANSPARENT)
    }

    pub fn set(&mut self, x: usize, y: usize, color: Color) {
        if let Some(elem) = self.0.get_mut(y).and_then(|row| row.get_mut(x)) {
            *elem = color;
        }
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        assert_eq!(mem::size_of::<Color>(), 4);
        // Safety: Color can safely be read/written as bytes, and has no invalid values
        unsafe {
            let len = mem::size_of_val(&self.0);
            slice::from_raw_parts_mut(&mut self.0 as *mut _ as *mut u8, len)
        }
    }
}
