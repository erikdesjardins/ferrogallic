use crate::util::NeqAssign;
use ferrogallic_shared::api::game::{Player, PlayerStatus};
use ferrogallic_shared::domain::UserId;
use std::collections::BTreeMap;
use std::sync::Arc;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub players: Arc<BTreeMap<UserId, Player>>,
}

pub struct Players {
    props: Props,
}

impl Component for Players {
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
        let players = self
            .props
            .players
            .values()
            .map(|player| {
                html! {
                    <li>
                        {&player.nick}
                        <ul>
                            <li>{"Score: "}{player.score}</li>
                            <li>{"Status: "}{match player.status {
                                PlayerStatus::Connected => "connected",
                                PlayerStatus::Disconnected => "disconnected",
                            }}</li>
                        </ul>
                    </li>
                }
            })
            .collect::<Html>();

        html! {
            <ul class="tree-view" style="height: 100%; overflow-y: scroll">
                {players}
            </ul>
        }
    }
}
