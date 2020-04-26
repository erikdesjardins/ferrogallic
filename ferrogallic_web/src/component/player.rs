use crate::util::NeqAssign;
use ferrogallic_shared::api::game;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub player: game::Player,
}

pub struct Player {
    props: Props,
}

impl Component for Player {
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
        let player = &self.props.player;
        html! {
            <p>
                {&player.nickname}{" ("}{player.score}{")"}
                {match player.status {
                     game::PlayerStatus::Connected => html!{},
                     game::PlayerStatus::Disconnected => html! {
                        {" - disconnected..."}
                     },
                }}
            </p>
        }
    }
}
