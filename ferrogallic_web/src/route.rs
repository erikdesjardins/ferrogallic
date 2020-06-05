use ferrogallic_shared::domain::{Lobby, Nickname};
use percent_encoding::{percent_decode, percent_encode, AsciiSet, CONTROLS};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use yew_router::Switch;

#[derive(Clone, Switch)]
pub enum AppRoute {
    #[to = "/join/{lobby}/as/{nick}"]
    InGame {
        lobby: UrlEncoded<Lobby>,
        nick: UrlEncoded<Nickname>,
    },
    #[to = "/join/{lobby}"]
    ChooseName { lobby: UrlEncoded<Lobby> },
    #[to = "/create"]
    Create,
}

#[derive(Clone)]
pub struct UrlEncoded<T: FromStr>(pub T);

impl<T: FromStr> FromStr for UrlEncoded<T> {
    type Err = T::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(T::from_str(
            &percent_decode(s.as_bytes()).decode_utf8_lossy(),
        )?))
    }
}

impl<T: FromStr + Deref<Target = str>> fmt::Display for UrlEncoded<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const URL_ESCAPE_CHARS: AsciiSet = CONTROLS
            .add(b' ')
            .add(b'/')
            .add(b'\\')
            .add(b'?')
            .add(b'&')
            .add(b'=')
            .add(b'#')
            .add(b'*');
        fmt::Display::fmt(&percent_encode(self.0.as_bytes(), &URL_ESCAPE_CHARS), f)
    }
}
