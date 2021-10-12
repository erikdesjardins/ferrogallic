use crate::util::NeqAssign;
use boolinator::Boolinator;
use ferrogallic_shared::domain::Lowercase;
use itertools::{EitherOrBoth, Itertools};
use yew::{classes, html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties, PartialEq, Eq)]
pub struct Props {
    pub word: Lowercase,
    pub reveal: Reveal,
    pub guess: Lowercase,
}

pub struct GuessTemplate {
    props: Props,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Reveal {
    All,
    Spaces,
}

impl Component for GuessTemplate {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        use EitherOrBoth::*;

        let reveal_chars: fn(char) -> Template = match self.props.reveal {
            Reveal::All => |c| match c {
                ' ' => Template::Space,
                _ => Template::Exact(c),
            },
            Reveal::Spaces => |c| match c {
                ' ' => Template::Space,
                _ => Template::NonSpace,
            },
        };

        let template_chars = self.props.word.chars().map(reveal_chars);
        let guess_chars = self.props.guess.chars();

        let template = template_chars
            .zip_longest(guess_chars)
            .map(|entry| match entry {
                Both(template, guess) => {
                    let underlined = template.is_underlined().as_some("underlined");
                    let invalid = (!template.is_valid(guess)).as_some("invalid");
                    html! { <span class=classes!("guess-char", underlined, invalid)>{guess}</span> }
                }
                Left(template) => {
                    let underlined = template.is_underlined().as_some("underlined");
                    html! { <span class=classes!("guess-char", underlined)>{template.char()}</span> }
                }
                Right(guess) => {
                    html! { <span class=classes!("guess-char", "invalid")>{guess}</span> }
                }
            })
            .collect::<Html>();

        html! {
            <div class="guess-chars">{template}</div>
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Template {
    Space,
    NonSpace,
    Exact(char),
}

impl Template {
    fn is_underlined(self) -> bool {
        match self {
            Self::Space => false,
            Self::NonSpace => true,
            Self::Exact(_) => true,
        }
    }

    fn is_valid(self, c: char) -> bool {
        match self {
            Self::Space => c == ' ',
            Self::NonSpace => c != ' ',
            Self::Exact(e) => c == e,
        }
    }

    fn char(self) -> char {
        match self {
            Self::Space => ' ',
            Self::NonSpace => ' ',
            Self::Exact(e) => e,
        }
    }
}
