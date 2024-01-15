use crate::dom::InputEventExt;
use crate::page;
use ferrogallic_shared::domain::Lowercase;
use web_sys::{InputEvent, SubmitEvent};
use yew::{html, Callback, Component, Context, Html, Properties};

pub enum Msg {}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub game_link: Callback<page::in_game::Msg>,
    pub guess: Lowercase,
}

pub struct GuessInput {}

impl Component for GuessInput {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_change_guess = ctx.props().game_link.reform(|e: InputEvent| {
            page::in_game::Msg::SetGuess(Lowercase::new(e.target_value().trim()))
        });
        let on_submit = ctx.props().game_link.reform(|e: SubmitEvent| {
            e.prevent_default();
            page::in_game::Msg::SendGuess
        });
        html! {
            <form onsubmit={on_submit}>
                <input
                    type="text"
                    oninput={on_change_guess}
                    value={ctx.props().guess.to_string()}
                    style="width: 100%"
                />
            </form>
        }
    }
}
