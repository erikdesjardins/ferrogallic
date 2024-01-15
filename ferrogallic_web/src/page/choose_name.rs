use crate::dom::InputEventExt;
use crate::route::{AppRoute, UrlEncoded};
use ferrogallic_shared::domain::{Lobby, Nickname};
use web_sys::SubmitEvent;
use yew::{html, Component, Context, Html, InputEvent, Properties};
use yew_router::scope_ext::RouterScopeExt;

pub enum Msg {
    SetNick(Nickname),
    GoToLobby,
}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub lobby: Lobby,
}

pub struct ChooseName {
    nick: Nickname,
}

impl Component for ChooseName {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            nick: Nickname::new(""),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetNick(nick) => {
                self.nick = nick;
                true
            }
            Msg::GoToLobby => {
                if let Some(navigator) = ctx.link().navigator() {
                    navigator.push(&AppRoute::InGame {
                        lobby: UrlEncoded(ctx.props().lobby.clone()),
                        nick: UrlEncoded(self.nick.clone()),
                    });
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_change_nick = ctx
            .link()
            .callback(|e: InputEvent| Msg::SetNick(Nickname::new(e.target_value())));
        let on_join_game = ctx.link().callback(|e: SubmitEvent| {
            e.prevent_default();
            Msg::GoToLobby
        });
        html! {
            <div style="display: flex; justify-content: space-evenly; align-items: flex-start">
                <main class="window" style="min-width: 300px">
                    <div class="title-bar">
                        <div class="title-bar-text">{"Join Game - "}{&ctx.props().lobby}</div>
                    </div>
                    <article class="window-body">
                        <form onsubmit={on_join_game}>
                            <p class="field-row-stacked">
                                <label for="nickname">{"Nickname"}</label>
                                <input
                                    id="nickname"
                                    type="text"
                                    oninput={on_change_nick}
                                    value={self.nick.to_string()}
                                />
                            </p>
                            <section class="field-row" style="justify-content: flex-end">
                                <button disabled={self.nick.is_empty()}>
                                    {"Join"}
                                </button>
                            </section>
                        </form>
                    </article>
                </main>
            </div>
        }
    }
}
