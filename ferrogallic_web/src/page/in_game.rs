use crate::api::{WebSocketApiTask, WebSocketServiceExt};
use crate::app;
use crate::canvas::VirtualCanvas;
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
use yew::services::render::{RenderService, RenderTask};
use yew::services::websocket::{WebSocketService, WebSocketStatus};
use yew::{
    html, Callback, Component, ComponentLink, Html, MouseEvent, NodeRef, PointerEvent, Properties,
    ShouldRender,
};
use yew_router::route::Route;

pub enum Msg {
    Ignore,
    Message(Game),
    ConnStatus(WebSocketStatus),
    Pointer(PointerAction),
    Undo,
    Render,
    SetTool(Tool),
    SetColor(Color),
    SetGlobalError(Error),
}

pub enum PointerAction {
    Down((u16, u16)),
    Move((u16, u16)),
    Up((u16, u16)),
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
    render_service: RenderService,
    lobby: Lobby,
    nickname: Nickname,
    active_ws: Option<WebSocketApiTask<Game>>,
    scheduled_render: Option<RenderTask>,
    canvas_ref: NodeRef,
    canvas: Option<(VirtualCanvas, CanvasRenderingContext2d)>,
    pointer: PointerState,
    tool: Tool,
    color: Color,
    players: BTreeMap<UserId, Player>,
}

#[derive(Copy, Clone)]
enum PointerState {
    Up,
    Down { at: (u16, u16) },
}

impl Component for InGame {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            app_link: props.app_link,
            ws_service: WebSocketService::new(),
            render_service: RenderService::new(),
            lobby: props.lobby,
            nickname: props.nickname,
            active_ws: None,
            scheduled_render: None,
            canvas_ref: Default::default(),
            canvas: None,
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
                self.canvas = Some((VirtualCanvas::new(), context));
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
                Game::Heartbeat => false,
                Game::Players { players } => {
                    self.players = players;
                    true
                }
                Game::Canvas { event } => {
                    self.render_to_virtual(event);
                    false
                }
            },
            Msg::Pointer(action) => {
                let one_event;
                let two_events;
                let events: &[Canvas] = match (self.tool, action) {
                    (Tool::Pen(_), PointerAction::Down(at)) => {
                        self.pointer = PointerState::Down { at };
                        &[]
                    }
                    (Tool::Pen(width), PointerAction::Move(to)) => match self.pointer {
                        PointerState::Down { at: from } if to != from => {
                            self.pointer = PointerState::Down { at: to };
                            one_event = [Canvas::Line {
                                from,
                                to,
                                width,
                                color: self.color,
                            }];
                            &one_event
                        }
                        PointerState::Down { .. } | PointerState::Up => &[],
                    },
                    (Tool::Pen(width), PointerAction::Up(to)) => match self.pointer {
                        PointerState::Down { at: from } => {
                            self.pointer = PointerState::Up;
                            two_events = [
                                Canvas::Line {
                                    from,
                                    to,
                                    width,
                                    color: self.color,
                                },
                                Canvas::PushUndo,
                            ];
                            &two_events
                        }
                        PointerState::Up => &[],
                    },
                    (Tool::Fill, PointerAction::Down(at)) => {
                        two_events = [
                            Canvas::Fill {
                                at,
                                color: self.color,
                            },
                            Canvas::PushUndo,
                        ];
                        &two_events
                    }
                    (Tool::Fill, PointerAction::Move(_)) | (Tool::Fill, PointerAction::Up(_)) => {
                        &[]
                    }
                };
                for &event in events {
                    self.render_to_virtual(event);
                    if let Some(ws) = &mut self.active_ws {
                        ws.send_api(&GameReq::Canvas { event });
                    }
                }
                false
            }
            Msg::Undo => {
                let event = Canvas::PopUndo;
                self.render_to_virtual(event);
                if let Some(ws) = &mut self.active_ws {
                    ws.send_api(&GameReq::Canvas { event });
                }
                false
            }
            Msg::Render => {
                self.render_to_canvas();
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
        let on_pointer_down =
            self.handle_pointer_event_if(|e| e.buttons() == 1, PointerAction::Down);
        let on_pointer_move = self.handle_pointer_event(PointerAction::Move);
        let on_pointer_up = self.handle_pointer_event(PointerAction::Up);

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
                            <component::UndoToolbar game_link=self.link.clone()/>
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

impl InGame {
    fn handle_pointer_event(
        &self,
        f: impl Fn((u16, u16)) -> PointerAction + 'static,
    ) -> Callback<PointerEvent> {
        self.handle_pointer_event_if(|_| true, f)
    }

    fn handle_pointer_event_if(
        &self,
        pred: impl Fn(&PointerEvent) -> bool + 'static,
        f: impl Fn((u16, u16)) -> PointerAction + 'static,
    ) -> Callback<PointerEvent> {
        self.link.callback(move |e: PointerEvent| {
            if pred(&e) {
                match e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                    Some(target) => {
                        e.prevent_default();
                        let origin = target.get_bounding_client_rect();
                        Msg::Pointer(f((
                            (e.client_x() as u16).saturating_sub(origin.x() as u16),
                            (e.client_y() as u16).saturating_sub(origin.y() as u16),
                        )))
                    }
                    None => Msg::Ignore,
                }
            } else {
                Msg::Ignore
            }
        })
    }

    fn render_to_virtual(&mut self, event: Canvas) {
        if let Some((canvas, _)) = &mut self.canvas {
            canvas.handle_event(event);
            if let scheduled @ None = &mut self.scheduled_render {
                *scheduled = Some(
                    self.render_service
                        .request_animation_frame(self.link.callback(|_| Msg::Render)),
                );
            }
        }
    }

    fn render_to_canvas(&mut self) {
        self.scheduled_render = None;
        if let Some((canvas, context)) = &mut self.canvas {
            if let Err(e) = canvas.render_to(context) {
                log::warn!("Failed to render to canvas: {:?}", e);
            }
        }
    }
}
