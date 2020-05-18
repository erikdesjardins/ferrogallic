use crate::page;
use crate::util::NeqAssign;
use ferrogallic_shared::domain::Lowercase;
use yew::{html, Component, ComponentLink, Event, Html, InputData, Properties, ShouldRender};

pub enum Msg {
    Submit,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub guess: Lowercase,
}

pub struct GuessInput {
    link: ComponentLink<Self>,
    game_link: ComponentLink<page::InGame>,
    guess: Lowercase,
}

impl Component for GuessInput {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            game_link: props.game_link,
            guess: props.guess,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Submit => {
                if !self.guess.is_empty() {
                    self.game_link.send_message(page::in_game::Msg::SendGuess);
                }
                true
            }
        }
    }

    fn change(&mut self, Props { game_link, guess }: Self::Properties) -> ShouldRender {
        self.game_link = game_link;
        self.guess.neq_assign(guess)
    }

    fn view(&self) -> Html {
        let on_change_guess = self
            .game_link
            .callback(|e: InputData| page::in_game::Msg::SetGuess(Lowercase::new(e.value)));
        let on_submit = self.link.callback(|e: Event| {
            e.prevent_default();
            Msg::Submit
        });
        html! {
            <form onsubmit=on_submit>
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
