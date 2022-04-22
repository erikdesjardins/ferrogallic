use crate::api::fetch_api;
use crate::app;
use crate::dom::InputEventExt;
use crate::route::{AppRoute, UrlEncoded};
use anyhow::Error;
use ferrogallic_shared::api::lobby::RandomLobbyName;
use ferrogallic_shared::domain::Lobby;
use wasm_bindgen_futures::spawn_local;
use web_sys::InputEvent;
use yew::{html, Callback, Component, Context, FocusEvent, Html, Properties};
use yew_router::history::History;
use yew_router::scope_ext::RouterScopeExt;

pub enum Msg {
    SetCustomLobbyName(Lobby),
    SetGeneratedLobbyName(Lobby),
    GoToCustomLobby,
    GoToGeneratedLobby,
    SetGlobalError(Error),
}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub app_link: Callback<app::Msg>,
}

pub struct Create {
    custom_lobby_name: Lobby,
    generated_lobby_name: Lobby,
}

impl Component for Create {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            custom_lobby_name: Lobby::new(""),
            generated_lobby_name: Lobby::new(""),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetCustomLobbyName(lobby) => {
                self.custom_lobby_name = lobby;
                true
            }
            Msg::SetGeneratedLobbyName(lobby) => {
                self.generated_lobby_name = lobby;
                true
            }
            Msg::GoToCustomLobby => {
                if let Some(history) = ctx.link().history() {
                    history.push(AppRoute::ChooseName {
                        lobby: UrlEncoded(self.custom_lobby_name.clone()),
                    });
                }
                false
            }
            Msg::GoToGeneratedLobby => {
                if let Some(history) = ctx.link().history() {
                    history.push(AppRoute::ChooseName {
                        lobby: UrlEncoded(self.generated_lobby_name.clone()),
                    });
                }
                false
            }
            Msg::SetGlobalError(e) => {
                ctx.props().app_link.emit(app::Msg::SetError(e));
                false
            }
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let link = ctx.link().clone();
            spawn_local(async move {
                link.send_message(match fetch_api(&()).await {
                    Ok(RandomLobbyName { lobby }) => Msg::SetGeneratedLobbyName(lobby),
                    Err(e) => Msg::SetGlobalError(e.context("Failed to fetch lobby name")),
                });
            });
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_change_custom_lobby = ctx
            .link()
            .callback(|e: InputEvent| Msg::SetCustomLobbyName(Lobby::new(e.target_value())));
        let on_join_custom = ctx.link().callback(|e: FocusEvent| {
            e.prevent_default();
            Msg::GoToCustomLobby
        });
        let on_join_generated = ctx.link().callback(|e: FocusEvent| {
            e.prevent_default();
            Msg::GoToGeneratedLobby
        });
        html! {
            <main style="display: flex; justify-content: space-evenly; align-items: flex-start">
                <div class="window" style="min-width: 300px">
                    <div class="title-bar">
                        <div class="title-bar-text">{"Join Existing Game"}</div>
                    </div>
                    <article class="window-body">
                        <form onsubmit={on_join_custom}>
                            <p class="field-row-stacked">
                                <label for="new-lobby">{"Lobby name"}</label>
                                <input
                                    id="new-lobby"
                                    type="text"
                                    oninput={on_change_custom_lobby}
                                    value={self.custom_lobby_name.to_string()}
                                />
                            </p>
                            <section class="field-row" style="justify-content: flex-end">
                                <button disabled={self.custom_lobby_name.is_empty()}>
                                    {"Join"}
                                </button>
                            </section>
                        </form>
                    </article>
                </div>
                <div class="window" style="min-width: 300px">
                    <div class="title-bar">
                        <div class="title-bar-text">{"Create New Game"}</div>
                    </div>
                    <article class="window-body">
                        <form onsubmit={on_join_generated}>
                            <p class="field-row-stacked">
                                <label for="random-lobby">{"Random lobby name"}</label>
                                <input
                                    id="random-lobby"
                                    type="text"
                                    disabled=true
                                    value={self.generated_lobby_name.to_string()}
                                />
                            </p>
                            <section class="field-row" style="justify-content: flex-end">
                                <button>
                                    {"Create"}
                                </button>
                            </section>
                        </form>
                    </article>
                </div>
            </main>
        }
    }
}
