use crate::util::NeqAssign;
use ferrogallic_shared::api::game::Player;
use ferrogallic_shared::domain::{Guess, UserId};
use std::collections::BTreeMap;
use std::sync::Arc;
use web_sys::Element;
use yew::{html, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties, PartialEq)]
pub struct Props {
    pub players: Arc<BTreeMap<UserId, Player>>,
    pub guesses: Arc<Vec<Arc<Guess>>>,
}

pub struct GuessArea {
    props: Props,
    area_ref: NodeRef,
}

impl Component for GuessArea {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            props,
            area_ref: Default::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn rendered(&mut self, _first_render: bool) {
        if let Some(area) = self.area_ref.cast::<Element>() {
            area.set_scroll_top(i32::MAX);
        }
    }

    fn view(&self) -> Html {
        let guesses = self
            .props
            .guesses
            .iter()
            .map(|guess| html! { <guess::GuessLine players=self.props.players.clone() guess=guess.clone()/> })
            .collect::<Html>();

        html! {
            <ul ref=self.area_ref.clone() class="tree-view" style="height: 100%; overflow-y: scroll">{guesses}</ul>
        }
    }
}

mod guess {
    use super::*;

    pub enum Msg {}

    #[derive(Clone, Properties)]
    pub struct Props {
        pub players: Arc<BTreeMap<UserId, Player>>,
        pub guess: Arc<Guess>,
    }

    impl PartialEq for Props {
        fn eq(&self, Self { players, guess }: &Self) -> bool {
            Arc::ptr_eq(&self.players, players) && Arc::ptr_eq(&self.guess, guess)
        }
    }

    pub struct GuessLine {
        props: Props,
    }

    impl Component for GuessLine {
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

            match self.props.guess.as_ref() {
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
            }
        }
    }
}
