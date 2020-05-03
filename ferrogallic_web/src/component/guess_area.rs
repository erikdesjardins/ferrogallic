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
    pub guesses: Arc<Vec<Guess>>,
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
            .map(|guess| match guess {
                Guess::System(system) => html! {
                    <div>{system}</div>
                },
                Guess::Message(user_id, message) => html! {
                    <div>{"["}{format_user(*user_id)}{"] "}{message}</div>
                },
                Guess::NowChoosing(user_id) => html! {
                    <div>{"["}{format_user(*user_id)}{"] is choosing a word."}</div>
                },
                Guess::Guess(user_id, guess) => html! {
                    <div>{"["}{format_user(*user_id)}{"] guessed '"}{guess}{"'."}</div>
                },
                Guess::Correct(user_id) => html! {
                    <div>{"["}{format_user(*user_id)}{"] "}{" guessed correctly!"}</div>
                },
                Guess::EarnedPoints(user_id, points) => html! {
                    <div>{"["}{format_user(*user_id)}{"] earned "}{points}{" points."}</div>
                },
                Guess::TimeExpired => html! {
                    <div>{"Time's up!"}</div>
                },
            })
            .collect::<Html>();
        html! {
            <div class="guesses">{guesses}</div>
        }
    }
}
