use crate::util::NeqAssign;
use ferrogallic_shared::api::game::Player;
use ferrogallic_shared::domain::{Guess, UserId};
use std::collections::BTreeMap;
use std::sync::Arc;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties, PartialEq)]
pub struct Props {
    pub players: Arc<BTreeMap<UserId, Player>>,
    pub guesses: Arc<Vec<Arc<Guess>>>,
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
        let format_user = |user_id| {
            self.props
                .players
                .get(&user_id)
                .map(|p| &*p.nick)
                .unwrap_or("<unknown>")
        };
        let guesses = self
            .props
            .guesses
            .iter()
            .map(|guess| match guess.as_ref() {
                Guess::System(system) => html! {
                    <li>{"🖥️ "}{system}</li>
                },
                Guess::Message(user_id, message) => html! {
                    <li>{format_user(*user_id)}{": "}{message}</li>
                },
                Guess::NowChoosing(user_id) => html! {
                    <li>{"✨ "}{format_user(*user_id)}{" is choosing a word."}</li>
                },
                Guess::Guess(user_id, guess) => html! {
                    <li>{"❌ "}{format_user(*user_id)}{" guessed '"}{guess}{"'."}</li>
                },
                Guess::Correct(user_id) => html! {
                    <li>{"✔️ "}{format_user(*user_id)}{" guessed correctly!"}</li>
                },
                Guess::EarnedPoints(user_id, points) => html! {
                    <li>{"💵 "}{format_user(*user_id)}{" earned "}{points}{" points."}</li>
                },
                Guess::TimeExpired => html! {
                    <li>{"⏰ Time's up!"}</li>
                },
            })
            .collect::<Html>();
        html! {
            <ul class="tree-view" style="height: 100%; overflow-y: scroll">{guesses}</ul>
        }
    }
}
