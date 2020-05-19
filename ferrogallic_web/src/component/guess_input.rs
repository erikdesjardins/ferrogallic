use crate::page;
use crate::util::NeqAssign;
use ferrogallic_shared::domain::Lowercase;
use yew::{html, Component, ComponentLink, Event, Html, InputData, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub guess: Lowercase,
}

pub struct GuessInput {
    game_link: ComponentLink<page::InGame>,
    guess: Lowercase,
}

impl Component for GuessInput {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            game_link: props.game_link,
            guess: props.guess,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, Props { game_link, guess }: Self::Properties) -> ShouldRender {
        self.game_link = game_link;
        self.guess.neq_assign(guess)
    }

    fn view(&self) -> Html {
        let on_change_guess = self
            .game_link
            .callback(|e: InputData| page::in_game::Msg::SetGuess(Lowercase::new(e.value)));
        let on_submit = self.game_link.callback(|e: Event| {
            e.prevent_default();
            page::in_game::Msg::SendGuess
        });
        html! {
            <form onsubmit=on_submit>
                <input
                    type="text"
                    oninput=on_change_guess
                    value=&self.guess
                    style="width: 100%"
                />
            </form>
        }
    }
}
