use crate::api::{WebSocketApiTask, WebSocketServiceExt};
use crate::app;
use crate::canvas::CanvasRenderingContext2dExt;
use crate::component;
use crate::route::AppRoute;
use crate::util::NeqAssign;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Canvas, Game, GameReq, Player};
use ferrogallic_shared::config::{CANVAS_HEIGHT, CANVAS_WIDTH};
use ferrogallic_shared::domain::{Color, Lobby, Nickname, Tool, UserId};
use std::collections::BTreeMap;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};
use yew::services::websocket::{WebSocketService, WebSocketStatus};
use yew::{
    html, Component, ComponentLink, Html, MouseEvent, NodeRef, PointerEvent, Properties,
    ShouldRender,
};
use yew_router::route::Route;

pub enum Msg {
    Ignore,
    Message(Game),
    ConnStatus(WebSocketStatus),
    Pointer(PointerAction),
    SetTool(Tool),
    SetColor(Color),
    SetGlobalError(Error),
}

pub enum PointerAction {
    Down { x: u16, y: u16 },
    Move { x: u16, y: u16 },
    Up,
}

#[derive(Clone, Properties)]
pub struct Props {
    pub app_link: ComponentLink<app::App>,
    pub lobby: Lobby,
    pub nickname: Nickname,
}

pub struct InGame {
    link: ComponentLink<Self>,
    app_link: ComponentLink<app::App>,
    ws_service: WebSocketService,
    lobby: Lobby,
    nickname: Nickname,
    active_ws: Option<WebSocketApiTask<Game>>,
    canvas_ref: NodeRef,
    context: Option<CanvasRenderingContext2d>,
    pointer: PointerState,
    tool: Tool,
    color: Color,
    players: BTreeMap<UserId, Player>,
}

#[derive(Copy, Clone)]
enum PointerState {
    Up,
    Down,
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
            canvas_ref: Default::default(),
            context: None,
            pointer: PointerState::Up,
            tool: Default::default(),
            color: Default::default(),
            players: Default::default(),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        if let Some(canvas) = self.canvas_ref.cast::<HtmlCanvasElement>() {
            if let Some(context) = canvas
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
            {
                context.initialize();
                self.context = Some(context);
            }
        }

        let started_ws = self.ws_service.connect_api(
            &self.link,
            |res| match res {
                Ok(msg) => Msg::Message(msg),
                Err(e) => Msg::SetGlobalError(e),
            },
            Msg::ConnStatus,
        );
        match started_ws {
            Ok(task) => self.active_ws = Some(task),
            Err(e) => self
                .app_link
                .send_message(app::Msg::SetError(e.context("Failed to connect"))),
        }

        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Ignore => false,
            Msg::ConnStatus(status) => match status {
                WebSocketStatus::Opened => {
                    if let Some(ws) = &mut self.active_ws {
                        ws.send_api(&GameReq::Join {
                            lobby: self.lobby.clone(),
                            nickname: self.nickname.clone(),
                        });
                    }
                    false
                }
                WebSocketStatus::Closed => {
                    self.active_ws = None;
                    self.app_link
                        .send_message(app::Msg::SetError(anyhow!("Lost connection")));
                    false
                }
                WebSocketStatus::Error => {
                    self.active_ws = None;
                    self.app_link
                        .send_message(app::Msg::SetError(anyhow!("Error in websocket")));
                    false
                }
            },
            Msg::Message(msg) => match msg {
                Game::Players { players } => {
                    self.players = players;
                    true
                }
                Game::Canvas { event } => {
                    if let Some(context) = &self.context {
                        context.handle_event(event);
                    }
                    false
                }
            },
            Msg::Pointer(action) => {
                let event = match (self.tool, action) {
                    (Tool::Pen(width), PointerAction::Down { x, y }) => {
                        self.pointer = PointerState::Down;
                        Some(Canvas::LineStart {
                            x,
                            y,
                            width,
                            color: self.color,
                        })
                    }
                    (Tool::Pen(_), PointerAction::Move { x, y }) => {
                        if let PointerState::Down = self.pointer {
                            Some(Canvas::LineTo { x, y })
                        } else {
                            None
                        }
                    }
                    (Tool::Pen(_), PointerAction::Up) => {
                        self.pointer = PointerState::Up;
                        None
                    }
                    (Tool::Fill, PointerAction::Down { x, y }) => Some(Canvas::Fill {
                        x,
                        y,
                        color: self.color,
                    }),
                    (Tool::Fill, PointerAction::Move { .. }) | (Tool::Fill, PointerAction::Up) => {
                        None
                    }
                };
                if let Some(event) = event {
                    if let Some(context) = &self.context {
                        context.handle_event(event);
                    }
                    if let Some(ws) = &mut self.active_ws {
                        ws.send_api(&GameReq::Canvas { event });
                    }
                }
                false
            }
            Msg::SetTool(tool) => {
                self.tool = tool;
                true
            }
            Msg::SetColor(color) => {
                self.color = color;
                true
            }
            Msg::SetGlobalError(e) => {
                self.app_link.send_message(app::Msg::SetError(e));
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
        let lobby_url = Route::<()>::from(AppRoute::ChooseName {
            lobby: self.lobby.clone(),
        })
        .to_string();
        let no_leftclick = self.link.callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::Ignore
        });
        let on_pointer_down = self.link.callback(|e: PointerEvent| {
            if e.buttons() == 1 {
                e.prevent_default();
                match e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                    Some(target) => {
                        let origin = target.get_bounding_client_rect();
                        Msg::Pointer(PointerAction::Down {
                            x: (e.client_x() as u16).saturating_sub(origin.x() as u16),
                            y: (e.client_y() as u16).saturating_sub(origin.y() as u16),
                        })
                    }
                    None => Msg::Ignore,
                }
            } else {
                Msg::Ignore
            }
        });
        let on_pointer_move = self.link.callback(|e: PointerEvent| {
            match e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                Some(target) => {
                    let origin = target.get_bounding_client_rect();
                    Msg::Pointer(PointerAction::Move {
                        x: (e.client_x() as u16).saturating_sub(origin.x() as u16),
                        y: (e.client_y() as u16).saturating_sub(origin.y() as u16),
                    })
                }
                None => Msg::Ignore,
            }
        });
        let on_pointer_up = self
            .link
            .callback(|_: PointerEvent| Msg::Pointer(PointerAction::Up));

        html! {
            <fieldset>
                <legend><a href=lobby_url.as_str() onclick=no_leftclick>{&lobby_url}</a></legend>
                <article class="game-container">
                    <fieldset>
                        <legend>{"Players"}</legend>
                        <section class="player-container">
                            <div class="players">
                                {self.players.values().map(|player| html! {
                                    <component::Player player=player/>
                                }).collect::<Html>()}
                            </div>
                        </section>
                    </fieldset>
                    <fieldset>
                        <legend>{"Canvas"}</legend>
                        <canvas
                            ref=self.canvas_ref.clone()
                            onpointerdown=on_pointer_down
                            onpointermove=on_pointer_move
                            onpointerup=&on_pointer_up
                            onpointerleave=on_pointer_up
                            width=CANVAS_WIDTH
                            height=CANVAS_HEIGHT
                        />
                        <section class="toolbar-container">
                            <component::ColorToolbar game_link=self.link.clone() color=self.color/>
                            <component::ToolToolbar game_link=self.link.clone() tool=self.tool/>
                        </section>
                    </fieldset>
                    <fieldset>
                        <legend>{"Guesses"}</legend>
                        <section class="guess-container">
                            <div class="guesses">

                            </div>
                            <form>
                                <input
                                    type="text"
                                    value=""
                                />
                                <input
                                    type="submit"
                                    value="Guess"
                                />
                            </form>
                        </section>
                    </fieldset>
                </article>
            </fieldset>
        }
    }
}
