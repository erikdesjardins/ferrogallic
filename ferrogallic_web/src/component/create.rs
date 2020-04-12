use anyhow::Error;
use yew::services::fetch::{FetchService, FetchTask};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};
use yew_router::components::RouterAnchor;

use ferrogallic_api::RandomLobbyName;

use crate::api::FetchServiceExt;
use crate::component;
use crate::route::Route;

pub enum Msg {
    SetLobbyName(String),
    SetGlobalError(Error),
}

#[derive(Clone, Properties)]
pub struct Props {
    pub app_link: ComponentLink<component::App>,
}

pub struct Create {
    link: ComponentLink<Self>,
    app_link: ComponentLink<component::App>,
    fetch_service: FetchService,
    fetching_lobby_name: Option<FetchTask>,
    lobby_name: String,
}

impl Component for Create {
    type Message = Msg;
    type Properties = Props;

    fn create(Props { app_link }: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            app_link,
            fetch_service: FetchService::new(),
            fetching_lobby_name: None,
            lobby_name: "".to_string(),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        let started_fetch = self
            .fetch_service
            .fetch_api(&self.link, (), |res| match res {
                Ok(RandomLobbyName { lobby }) => Msg::SetLobbyName(lobby),
                Err(e) => Msg::SetGlobalError(e),
            });
        match started_fetch {
            Ok(task) => self.fetching_lobby_name = Some(task),
            Err(e) => self.link.send_message(Msg::SetGlobalError(e)),
        }
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetLobbyName(lobby) => {
                self.lobby_name = lobby;
                true
            }
            Msg::SetGlobalError(e) => {
                self.app_link.send_message(component::app::Msg::SetError(e));
                false
            }
        }
    }

    fn change(&mut self, Props { app_link }: Self::Properties) -> ShouldRender {
        self.app_link = app_link;
        false
    }

    fn view(&self) -> Html {
        let route = Route::ChooseName {
            lobby: self.lobby_name.clone(),
        };
        html! {
            <>
                <fieldset>
                    <legend>{"Join Game"}</legend>
                </fieldset>
                <fieldset>
                    <legend>{"New Game"}</legend>
                    <RouterAnchor<Route> route=route>
                        {&self.lobby_name}
                    </RouterAnchor<Route>>
                </fieldset>
            </>
        }
    }
}
