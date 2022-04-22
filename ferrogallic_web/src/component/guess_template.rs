use boolinator::Boolinator;
use ferrogallic_shared::domain::Lowercase;
use itertools::{EitherOrBoth, Itertools};
use yew::{classes, html, Component, Context, Html, Properties};

pub enum Msg {}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub word: Lowercase,
    pub reveal: Reveal,
    pub guess: Lowercase,
}

pub struct GuessTemplate {}

#[derive(Copy, Clone, PartialEq)]
pub enum Reveal {
    All,
    Spaces,
}

impl Component for GuessTemplate {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        use EitherOrBoth::*;

        let reveal_chars: fn(char) -> Template = match ctx.props().reveal {
            Reveal::All => |c| match c {
                ' ' => Template::Space,
                _ => Template::Exact(c),
            },
            Reveal::Spaces => |c| match c {
                ' ' => Template::Space,
                _ => Template::NonSpace,
            },
        };

        let template_chars = ctx.props().word.chars().map(reveal_chars);
        let guess_chars = ctx.props().guess.chars();

        let template = template_chars
            .zip_longest(guess_chars)
            .map(|entry| match entry {
                Both(template, guess) => {
                    let underlined = template.is_underlined().as_some("underlined");
                    let invalid = (!template.is_valid(guess)).as_some("invalid");
                    html! { <span class={classes!("guess-char", underlined, invalid)}>{guess}</span> }
                }
                Left(template) => {
                    let underlined = template.is_underlined().as_some("underlined");
                    html! { <span class={classes!("guess-char", underlined)}>{template.char()}</span> }
                }
                Right(guess) => {
                    html! { <span class={classes!("guess-char", "invalid")}>{guess}</span> }
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
