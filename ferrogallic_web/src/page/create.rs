use crate::api::FetchServiceExt;
use crate::app;
use crate::route::AppRoute;
use anyhow::Error;
use ferrogallic_shared::api::lobby::RandomLobbyName;
use ferrogallic_shared::domain::Lobby;
use yew::agent::{Dispatched, Dispatcher};
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Event, Html, InputData, Properties, ShouldRender};
use yew_router::agent::{RouteAgent, RouteRequest};
use yew_router::components::RouterAnchor;
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
    fetch_service: FetchService,
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
            fetch_service: FetchService::new(),
            custom_lobby_name: Lobby::new("".to_string()),
            fetching_generated_lobby_name: None,
            generated_lobby_name: Lobby::new("".to_string()),
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
                        lobby: self.custom_lobby_name.clone(),
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
            let started_fetch = self
                .fetch_service
                .fetch_api(&self.link, &(), |res| match res {
                    Ok(RandomLobbyName { lobby }) => Msg::SetGeneratedLobbyName(lobby),
                    Err(e) => Msg::SetGlobalError(e),
                });
            match started_fetch {
                Ok(task) => self.fetching_generated_lobby_name = Some(task),
                Err(e) => self.app_link.send_message(app::Msg::SetError(e)),
            }
        }
    }

    fn view(&self) -> Html {
        let on_change_custom_lobby = self
            .link
            .callback(|e: InputData| Msg::SetCustomLobbyName(Lobby::new(e.value)));
        let on_join_game = self.link.callback(|e: Event| {
            e.prevent_default();
            Msg::GoToCustomLobby
        });
        let generated_lobby = AppRoute::ChooseName {
            lobby: self.generated_lobby_name.clone(),
        };
        html! {
            <>
                <fieldset>
                    <legend>{"Join Game"}</legend>
                    <form onsubmit=on_join_game>
                        <input
                            type="text"
                            placeholder="Lobby"
                            oninput=on_change_custom_lobby
                            value=&self.custom_lobby_name
                        />
                        <input
                            type="submit"
                            value="Go"
                            disabled=self.custom_lobby_name.is_empty()
                        />
                    </form>
                </fieldset>
                <fieldset>
                    <legend>{"New Game"}</legend>
                    <RouterAnchor<AppRoute> route=generated_lobby>
                        {if self.generated_lobby_name.is_empty() {
                            "..."
                        } else {
                            &self.generated_lobby_name
                        }}
                    </RouterAnchor<AppRoute>>
                </fieldset>
            </>
        }
    }
}
