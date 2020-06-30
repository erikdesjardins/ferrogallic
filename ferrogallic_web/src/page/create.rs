use crate::api::FetchServiceExt;
use crate::app;
use crate::route::{AppRoute, UrlEncoded};
use anyhow::Error;
use ferrogallic_shared::api::lobby::RandomLobbyName;
use ferrogallic_shared::domain::Lobby;
use yew::agent::{Dispatched, Dispatcher};
use yew::services::fetch::FetchTask;
use yew::{html, Component, ComponentLink, FocusEvent, Html, InputData, Properties, ShouldRender};
use yew_router::agent::{RouteAgent, RouteRequest};
use yew_router::components::RouterButton;
use yew_router::route::Route;

pub enum Msg {
    SetCustomLobbyName(Lobby),
    GoToCustomLobby,
    SetGeneratedLobbyName(Lobby),
    SetGlobalError(Error),
}

#[derive(Clone, Properties)]
pub struct Props {
    pub app_link: ComponentLink<app::App>,
}

pub struct Create {
    link: ComponentLink<Self>,
    app_link: ComponentLink<app::App>,
    router: Dispatcher<RouteAgent>,
    custom_lobby_name: Lobby,
    fetching_generated_lobby_name: Option<FetchTask>,
    generated_lobby_name: Lobby,
}

impl Component for Create {
    type Message = Msg;
    type Properties = Props;

    fn create(Props { app_link }: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            app_link,
            router: RouteAgent::dispatcher(),
            custom_lobby_name: Lobby::new(""),
            fetching_generated_lobby_name: None,
            generated_lobby_name: Lobby::new(""),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetCustomLobbyName(lobby) => {
                self.custom_lobby_name = lobby;
                true
            }
            Msg::GoToCustomLobby => {
                self.router.send(RouteRequest::ChangeRoute(Route::from(
                    AppRoute::ChooseName {
                        lobby: UrlEncoded(self.custom_lobby_name.clone()),
                    },
                )));
                false
            }
            Msg::SetGeneratedLobbyName(lobby) => {
                self.generated_lobby_name = lobby;
                true
            }
            Msg::SetGlobalError(e) => {
                self.app_link.send_message(app::Msg::SetError(e));
                false
            }
        }
    }

    fn change(&mut self, Props { app_link }: Self::Properties) -> ShouldRender {
        self.app_link = app_link;
        false
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            let started_fetch = FetchServiceExt::fetch_api(&self.link, &(), |res| match res {
                Ok(RandomLobbyName { lobby }) => Msg::SetGeneratedLobbyName(lobby),
                Err(e) => Msg::SetGlobalError(e.context("Failed to receive lobby name")),
            });
            match started_fetch {
                Ok(task) => self.fetching_generated_lobby_name = Some(task),
                Err(e) => self
                    .app_link
                    .send_message(app::Msg::SetError(e.context("Failed to fetch lobby name"))),
            }
        }
    }

    fn view(&self) -> Html {
        let on_change_custom_lobby = self
            .link
            .callback(|e: InputData| Msg::SetCustomLobbyName(Lobby::new(e.value)));
        let on_join_game = self.link.callback(|e: FocusEvent| {
            e.prevent_default();
            Msg::GoToCustomLobby
        });
        let generated_lobby = AppRoute::ChooseName {
            lobby: UrlEncoded(self.generated_lobby_name.clone()),
        };
        html! {
            <main style="display: flex; justify-content: space-evenly; align-items: flex-start">
                <div class="window" style="min-width: 300px">
                    <div class="title-bar">
                        <div class="title-bar-text">{"Join Existing Game"}</div>
                    </div>
                    <article class="window-body">
                        <form onsubmit=on_join_game>
                            <p class="field-row-stacked">
                                <label for="new-lobby">{"Lobby name"}</label>
                                <input
                                    id="new-lobby"
                                    type="text"
                                    oninput=on_change_custom_lobby
                                    value=&self.custom_lobby_name
                                />
                            </p>
                            <section class="field-row" style="justify-content: flex-end">
                                <button disabled=self.custom_lobby_name.is_empty()>
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
                        <p class="field-row-stacked">
                            <label for="random-lobby">{"Random lobby name"}</label>
                            <input
                                id="random-lobby"
                                type="text"
                                disabled=true
                                value=&self.generated_lobby_name
                            />
                        </p>
                        <section class="field-row" style="justify-content: flex-end">
                            <RouterButton<AppRoute> route=generated_lobby>
                                {"Create"}
                            </RouterButton<AppRoute>>
                        </section>
                    </article>
                </div>
            </main>
        }
    }
}
