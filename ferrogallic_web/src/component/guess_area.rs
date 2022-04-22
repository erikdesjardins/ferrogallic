use crate::util::ArcPtrEq;
use ferrogallic_shared::api::game::Player;
use ferrogallic_shared::domain::{Guess, UserId};
use std::collections::BTreeMap;
use web_sys::Element;
use yew::{html, Component, Context, Html, NodeRef, Properties};

pub enum Msg {}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub players: ArcPtrEq<BTreeMap<UserId, Player>>,
    pub guesses: ArcPtrEq<Vec<Guess>>,
}

pub struct GuessArea {
    area_ref: NodeRef,
}

impl Component for GuessArea {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            area_ref: Default::default(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if let Some(area) = self.area_ref.cast::<Element>() {
            area.set_scroll_top(i32::MAX);
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let guesses = ctx
            .props()
            .guesses
            .iter()
            .map(|guess| html! { <guess::GuessLine players={ctx.props().players.clone()} guess={guess.clone()}/> })
            .collect::<Html>();

        html! {
            <ul ref={self.area_ref.clone()} class="tree-view" style="height: 100%; overflow-y: scroll">{guesses}</ul>
        }
    }
}

mod guess {
    use super::*;

    pub enum Msg {}

    #[derive(PartialEq, Properties)]
    pub struct Props {
        pub players: ArcPtrEq<BTreeMap<UserId, Player>>,
        pub guess: Guess,
    }

    pub struct GuessLine {}

    impl Component for GuessLine {
        type Message = Msg;
        type Properties = Props;

        fn create(_ctx: &Context<Self>) -> Self {
            Self {}
        }

        fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
            match msg {}
        }

        fn view(&self, ctx: &Context<Self>) -> Html {
            let nickname = |user_id| {
                ctx.props()
                    .players
                    .get(&user_id)
                    .map(|p| &*p.nick)
                    .unwrap_or("<unknown>")
            };

            let rank_emoji = |rank| match rank {
                1 => "🏆",
                2 | 3 => "🏅",
                _ => "🎖️",
            };

            match &ctx.props().guess {
                Guess::System(system) => html! {
                    <li>{"🖥️ "}{system}</li>
                },
                Guess::Help => html! {
                    <>
                    <li>{"❓ Type 'start' to start the game."}</li>
                    <li>{"❓ Type 'rounds <number>' to change number of rounds."}</li>
                    <li>{"❓ Type 'seconds <number>' to change guess timer."}</li>
                    </>
                },
                Guess::Message(user_id, message) => html! {
                    <li>{nickname(*user_id)}{": "}{message}</li>
                },
                Guess::NowChoosing(user_id) => html! {
                    <li>{"✨ "}{nickname(*user_id)}{" is choosing a word."}</li>
                },
                Guess::NowDrawing(user_id) => html! {
                    <li>{"🖌️ "}{nickname(*user_id)}{" is drawing!"}</li>
                },
                Guess::Guess(user_id, guess) => html! {
                    <li>{"❌ "}{nickname(*user_id)}{" guessed '"}{guess}{"'."}</li>
                },
                Guess::CloseGuess(guess) => html! {
                    <li>{"🤏 '"}{guess}{"' is close!"}</li>
                },
                Guess::Correct(user_id) => html! {
                    <li>{"✔️ "}{nickname(*user_id)}{" guessed correctly!"}</li>
                },
                Guess::EarnedPoints(user_id, points) => html! {
                    <li>{"💵 "}{nickname(*user_id)}{" earned "}{points}{" points."}</li>
                },
                Guess::TimeExpired(word) => html! {
                    <li>{"⏰ Time's up! The word was '"}{word}{"'."}</li>
                },
                Guess::GameOver => html! {
                    <li>{"🎮 Game over!"}</li>
                },
                Guess::FinalScore {
                    rank,
                    user_id,
                    score,
                } => html! {
                    <li>{rank_emoji(*rank)}{" (#"}{rank}{") "}{nickname(*user_id)}{" with "}{score}{" points."}</li>
                },
            }
        }
    }
}
