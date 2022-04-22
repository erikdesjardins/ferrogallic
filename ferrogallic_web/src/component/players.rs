use crate::page;
use ferrogallic_shared::api::game::{Player, PlayerStatus};
use ferrogallic_shared::domain::UserId;
use std::collections::BTreeMap;
use std::sync::Arc;
use yew::{html, Callback, Component, Context, Html, MouseEvent, Properties};

pub enum Msg {}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub game_link: Callback<page::in_game::Msg>,
    pub players: Arc<BTreeMap<UserId, Player>>,
}

pub struct Players {}

impl Component for Players {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let player_rankings = Player::rankings(ctx.props().players.as_ref())
            .take_while(|(_, _, player)| player.score > 0)
            .map(|(rank, uid, _)| (uid, rank))
            .collect::<BTreeMap<_, _>>();
        let players = ctx
            .props()
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
                        let on_remove = ctx.props().game_link.reform(move |e: MouseEvent| {
                            e.prevent_default();
                            page::in_game::Msg::RemovePlayer(user_id, epoch)
                        });
                        html! {
                            <>
                                {"disconnected "}
                                <a href="#" onclick={on_remove}>{"(remove)"}</a>
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
