use crate::config::{CANVAS_HEIGHT, CANVAS_WIDTH};
use serde::{Deserialize, Serialize};
use std::alloc;
use std::collections::hash_map::DefaultHasher;
use std::convert::Infallible;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops::Deref;
use std::ptr;
use std::slice;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct UserId(u64);

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Nickname(Box<str>);

impl Nickname {
    pub fn new(nick: impl Into<Box<str>>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Lobby(Box<str>);

impl Lobby {
    pub fn new(lobby: impl Into<Box<str>>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Guess {
    System(Box<str>),
    Message(UserId, Box<str>),
    Guess(UserId, Box<str>),
    Correct(UserId),
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum LineWidth {
    Small,
    Medium,
    Large,
    Extra,
}

impl Default for LineWidth {
    fn default() -> Self {
        Self::Small
    }
}

impl LineWidth {
    pub const ALL: [Self; 4] = [Self::Small, Self::Medium, Self::Large, Self::Extra];

    pub fn scanlines(self) -> &'static [u16] {
        match self {
            Self::Small => {
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

    pub fn text(self) -> &'static str {
        match self {
            Self::Small => "1",
            Self::Medium => "2",
            Self::Large => "4",
            Self::Extra => "7",
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
    pub const ALL: [Self; 5] = [
        Self::Pen(LineWidth::Small),
        Self::Pen(LineWidth::Medium),
        Self::Pen(LineWidth::Large),
        Self::Pen(LineWidth::Extra),
        Self::Fill,
    ];
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
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

    pub const ALL: [Self; 22] = [
        Self::WHITE,                 // White
        Self::BLACK,                 // Black
        Self::new(0xC1, 0xC1, 0xC1), // Grey
        Self::new(0x4C, 0x4C, 0x4C), // DarkGrey
        Self::new(0xEF, 0x13, 0x0B), // Red
        Self::new(0x74, 0x0B, 0x07), // DarkRed
        Self::new(0xFF, 0x71, 0x00), // Orange
        Self::new(0xC2, 0x38, 0x00), // DarkOrange
        Self::new(0xFF, 0xE4, 0x00), // Yellow
        Self::new(0xE8, 0xA2, 0x00), // DarkYellow
        Self::new(0x00, 0xCC, 0x00), // Green
        Self::new(0x00, 0x55, 0x10), // DarkGreen
        Self::new(0x00, 0xB2, 0xFF), // Blue
        Self::new(0x00, 0x56, 0x9E), // DarkBlue
        Self::new(0x23, 0x1F, 0xD3), // Indigo
        Self::new(0x0E, 0x08, 0x65), // DarkIndigo
        Self::new(0xA3, 0x00, 0xBA), // Violet
        Self::new(0x55, 0x00, 0x69), // DarkViolet
        Self::new(0xD3, 0x7C, 0xAA), // Pink
        Self::new(0xA7, 0x55, 0x74), // DarkPink
        Self::new(0xA0, 0x52, 0x2D), // Brown
        Self::new(0x63, 0x30, 0x0D), // DarkBrown
    ];

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { a: 0xff, r, g, b }
    }

    pub fn css(self) -> String {
        format!("rgb({},{},{})", self.r, self.g, self.b)
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
