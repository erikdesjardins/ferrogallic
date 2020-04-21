use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::convert::Infallible;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct UserId(u64);

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Nickname(String);

impl Nickname {
    pub fn new(nickname: String) -> Self {
        Self(nickname)
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
        Ok(Self(String::from(s)))
    }
}

impl fmt::Display for Nickname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Lobby(String);

impl Lobby {
    pub fn new(nickname: String) -> Self {
        Self(nickname)
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
        Ok(Self(String::from(s)))
    }
}

impl fmt::Display for Lobby {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
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

    pub fn px(self) -> u8 {
        match self {
            Self::Small => 2,
            Self::Medium => 4,
            Self::Large => 8,
            Self::Extra => 16,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Self::Small => "2",
            Self::Medium => "4",
            Self::Large => "8",
            Self::Extra => "16",
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
pub enum Color {
    White,
    Black,
    Grey,
    DarkGrey,
    Red,
    DarkRed,
    Orange,
    DarkOrange,
    Yellow,
    DarkYellow,
    Green,
    DarkGreen,
    Blue,
    DarkBlue,
    Indigo,
    DarkIndigo,
    Violet,
    DarkViolet,
    Pink,
    DarkPink,
    Brown,
    DarkBrown,
}

impl Default for Color {
    fn default() -> Self {
        Self::Black
    }
}

impl Color {
    pub const ALL: [Self; 22] = [
        Self::White,
        Self::Black,
        Self::Grey,
        Self::DarkGrey,
        Self::Red,
        Self::DarkRed,
        Self::Orange,
        Self::DarkOrange,
        Self::Yellow,
        Self::DarkYellow,
        Self::Green,
        Self::DarkGreen,
        Self::Blue,
        Self::DarkBlue,
        Self::Indigo,
        Self::DarkIndigo,
        Self::Violet,
        Self::DarkViolet,
        Self::Pink,
        Self::DarkPink,
        Self::Brown,
        Self::DarkBrown,
    ];

    pub fn css(self) -> &'static str {
        match self {
            Self::White => "#FFFFFF",
            Self::Black => "#000000",
            Self::Grey => "#C1C1C1",
            Self::DarkGrey => "#4C4C4C",
            Self::Red => "#EF130B",
            Self::DarkRed => "#740B07",
            Self::Orange => "#FF7100",
            Self::DarkOrange => "#C23800",
            Self::Yellow => "#FFE400",
            Self::DarkYellow => "#E8A200",
            Self::Green => "#00CC00",
            Self::DarkGreen => "#005510",
            Self::Blue => "#00B2FF",
            Self::DarkBlue => "#00569E",
            Self::Indigo => "#231FD3",
            Self::DarkIndigo => "#0E0865",
            Self::Violet => "#A300BA",
            Self::DarkViolet => "#550069",
            Self::Pink => "#D37CAA",
            Self::DarkPink => "#A75574",
            Self::Brown => "#A0522D",
            Self::DarkBrown => "#63300D",
        }
    }

    #[allow(clippy::mixed_case_hex_literals, clippy::unreadable_literal)]
    pub fn argb(self) -> u32 {
        match self {
            Self::White => 0xffFFFFFF,
            Self::Black => 0xff000000,
            Self::Grey => 0xffC1C1C1,
            Self::DarkGrey => 0xff4C4C4C,
            Self::Red => 0xffEF130B,
            Self::DarkRed => 0xff740B07,
            Self::Orange => 0xffFF7100,
            Self::DarkOrange => 0xffC23800,
            Self::Yellow => 0xffFFE400,
            Self::DarkYellow => 0xffE8A200,
            Self::Green => 0xff00CC00,
            Self::DarkGreen => 0xff005510,
            Self::Blue => 0xff00B2FF,
            Self::DarkBlue => 0xff00569E,
            Self::Indigo => 0xff231FD3,
            Self::DarkIndigo => 0xff0E0865,
            Self::Violet => 0xffA300BA,
            Self::DarkViolet => 0xff550069,
            Self::Pink => 0xffD37CAA,
            Self::DarkPink => 0xffA75574,
            Self::Brown => 0xffA0522D,
            Self::DarkBrown => 0xff63300D,
        }
    }
}
