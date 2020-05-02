use crate::util::NeqAssign;
use ferrogallic_shared::api::game::Player;
use ferrogallic_shared::domain::{Guess, UserId};
use std::collections::BTreeMap;
use std::rc::Rc;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties, PartialEq)]
pub struct Props {
    pub players: Rc<BTreeMap<UserId, Player>>,
    pub guesses: Rc<Vec<Guess>>,
}

pub struct GuessArea {
    props: Props,
}

impl Component for GuessArea {
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
        let guesses = self
            .props
            .guesses
            .iter()
            .map(|guess| match guess {
                Guess::System(system) => html! {
                    <div>{"[SYSTEM] "}{system}</div>
                },
                Guess::Message(message) => html! {
                    <div>{message}</div>
                },
                Guess::Guess(guess) => html! {
                    <div>{"[guess] "}{guess}</div>
                },
                Guess::Correct(user_id) => {
                    let player = self
                        .props
                        .players
                        .get(&user_id)
                        .map(|p| &*p.nick)
                        .unwrap_or("<unknown>");
                    html! { <div>{player}{" guessed correctly!"}</div> }
                }
            })
            .collect::<Html>();
        html! {
            <div class="guesses">
                {guesses}
            </div>
        }
    }
}
