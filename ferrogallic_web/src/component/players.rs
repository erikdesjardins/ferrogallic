use crate::page;
use crate::util::NeqAssign;
use ferrogallic_shared::api::game::{Player, PlayerStatus};
use ferrogallic_shared::domain::UserId;
use std::collections::BTreeMap;
use std::sync::Arc;
use yew::{html, Component, ComponentLink, Html, MouseEvent, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub players: Arc<BTreeMap<UserId, Player>>,
}

pub struct Players {
    game_link: ComponentLink<page::InGame>,
    players: Arc<BTreeMap<UserId, Player>>,
}

impl Component for Players {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            game_link: props.game_link,
            players: props.players,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, Props { game_link, players }: Self::Properties) -> ShouldRender {
        self.game_link = game_link;
        self.players.neq_assign(players)
    }

    fn view(&self) -> Html {
        let player_rankings = Player::rankings(self.players.as_ref())
            .take_while(|(_, _, player)| player.score > 0)
            .map(|(rank, uid, _)| (uid, rank))
            .collect::<BTreeMap<_, _>>();
        let players = self
            .players
            .iter()
            .map(|(&user_id, player)| {
                let ranking = match player_rankings.get(&user_id) {
                    Some(rank) => html! { <>{" (#"}{rank}{")"}</> },
                    None => html! {},
                };
                let status = match player.status {
                    PlayerStatus::Connected => html! { "connected" },
                    PlayerStatus::Disconnected => {
                        let epoch = player.epoch;
                        let on_remove = self.game_link.callback(move |e: MouseEvent| {
                            e.prevent_default();
                            page::in_game::Msg::RemovePlayer(user_id, epoch)
                        });
                        html! {
                            <>
                                {"disconnected "}
                                <a href="#" onclick=on_remove>{"(remove)"}</a>
                            </>
                        }
                    }
                };
                html! {
                    <li>
                        {&player.nick}
                        <ul>
                            <li>{"Score: "}{player.score}{ranking}</li>
                            <li>{"Status: "}{status}</li>
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
