use anyhow::{anyhow, Error};
use yew::services::websocket::{WebSocketService, WebSocketStatus};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use ferrogallic_shared::api::{Game, GameReq};

use crate::api::{WebSocketApiTask, WebSocketServiceExt};
use crate::component;
use crate::util::NeqAssign;

pub enum Msg {
    Ignore,
    WsOpened,
    WsClosed,
    WsError,
    SetGlobalError(Error),
}

#[derive(Clone, Properties)]
pub struct Props {
    pub app_link: ComponentLink<component::App>,
    pub lobby: String,
    pub nickname: String,
}

pub struct InGame {
    link: ComponentLink<Self>,
    app_link: ComponentLink<component::App>,
    ws_service: WebSocketService,
    lobby: String,
    nickname: String,
    active_ws: Option<WebSocketApiTask<Game>>,
}

impl Component for InGame {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            app_link: props.app_link,
            ws_service: WebSocketService::new(),
            lobby: props.lobby,
            nickname: props.nickname,
            active_ws: None,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        let started_ws = self.ws_service.connect_api(
            &self.link,
            |res| match res {
                Ok(Game::EchoJoin { lobby, nickname }) => {
                    log::info!("join msg: {} {}", lobby, nickname);
                    Msg::Ignore
                }
                Err(e) => Msg::SetGlobalError(e),
            },
            |status| match status {
                WebSocketStatus::Opened => Msg::WsOpened,
                WebSocketStatus::Closed => Msg::WsClosed,
                WebSocketStatus::Error => Msg::WsError,
            },
        );
        match started_ws {
            Ok(task) => self.active_ws = Some(task),
            Err(e) => self
                .app_link
                .send_message(component::app::Msg::SetError(anyhow!(e.to_string()))),
        }
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Ignore => false,
            Msg::WsOpened => {
                if let Some(ws) = &mut self.active_ws {
                    ws.send_api(&GameReq::Join {
                        lobby: self.lobby.clone(),
                        nickname: self.nickname.clone(),
                    });
                }
                false
            }
            Msg::WsClosed => {
                self.active_ws = None;
                log::info!("Closed");
                false
            }
            Msg::WsError => {
                self.active_ws = None;
                self.app_link
                    .send_message(component::app::Msg::SetError(anyhow!("Error in websocket")));
                false
            }
            Msg::SetGlobalError(e) => {
                self.app_link.send_message(component::app::Msg::SetError(e));
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        let Props {
            app_link,
            lobby,
            nickname,
        } = props;
        self.app_link = app_link;
        self.lobby.neq_assign(lobby) | self.nickname.neq_assign(nickname)
    }

    fn view(&self) -> Html {
        html! {
            <p>{"In game page"}</p>
        }
    }
}
