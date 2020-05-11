use crate::page;
use crate::util::{NeqAssign, StrExt};
use ferrogallic_shared::domain::Lowercase;
use itertools::{EitherOrBoth, Itertools};
use std::mem;
use yew::{html, Component, ComponentLink, Event, Html, InputData, Properties, ShouldRender};

pub enum Msg {
    SetGuess(Lowercase),
    Submit,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub guess_template: Option<Template>,
}

pub struct GuessInput {
    link: ComponentLink<Self>,
    game_link: ComponentLink<page::InGame>,
    guess_template: Option<Template>,
    guess: Lowercase,
}

impl Component for GuessInput {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            game_link: props.game_link,
            guess_template: props.guess_template,
            guess: Default::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetGuess(guess) => {
                self.guess = guess;
                true
            }
            Msg::Submit => {
                let guess = mem::take(&mut self.guess);
                if !guess.is_empty() {
                    self.game_link
                        .send_message(page::in_game::Msg::SendGuess(guess));
                }
                true
            }
        }
    }

    fn change(
        &mut self,
        Props {
            game_link,
            guess_template,
        }: Self::Properties,
    ) -> ShouldRender {
        self.game_link = game_link;
        self.guess_template.neq_assign(guess_template)
    }

    fn view(&self) -> Html {
        let on_change_guess = self
            .link
            .callback(|e: InputData| Msg::SetGuess(Lowercase::new(e.value)));
        let on_submit = self.link.callback(|e: Event| {
            e.prevent_default();
            Msg::Submit
        });
        html! {
            <form onsubmit=on_submit style="width: 100%">
                {match &self.guess_template {
                    Some(guess_template) => self.render_template(&guess_template.0),
                    None => html! {},
                }}
                <input
                    type="text"
                    placeholder="Guess"
                    oninput=on_change_guess
                    value=&self.guess
                    style="width: 100%"
                />
            </form>
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Template(Box<[TemplateInner]>);

impl Template {
    pub fn reveal_spaces(template: &str) -> Self {
        Self(
            template
                .chars()
                .map(|c| match c {
                    ' ' => TemplateInner::Space,
                    _ => TemplateInner::NonSpace,
                })
                .collect(),
        )
    }

    pub fn reveal_all(template: &str) -> Self {
        Self(
            template
                .chars()
                .map(|c| match c {
                    ' ' => TemplateInner::Space,
                    _ => TemplateInner::Exact(c),
                })
                .collect(),
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum TemplateInner {
    Space,
    NonSpace,
    Exact(char),
}

impl TemplateInner {
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

impl GuessInput {
    fn render_template(&self, guess_template: &[TemplateInner]) -> Html {
        use EitherOrBoth::*;
        let template = guess_template
            .iter()
            .zip_longest(self.guess.chars())
            .map(|entry| match entry {
                Both(template, guess) => {
                    let underlined = "underlined".class_if(template.is_underlined());
                    let invalid = "invalid".class_if(!template.is_valid(guess));
                    html! { <span class=("guess-char", underlined, invalid)>{guess}</span> }
                }
                Left(template) => {
                    let underlined = "underlined".class_if(template.is_underlined());
                    html! { <span class=("guess-char", underlined)>{template.char()}</span> }
                }
                Right(guess) => {
                    html! { <span class=("guess-char", "invalid")>{guess}</span> }
                }
            })
            .collect::<Html>();
        html! {
            <div class="guess-chars">{template}</div>
        }
    }
}
