use crate::api::{WebSocketApiTask, WebSocketServiceExt};
use crate::app;
use crate::canvas::VirtualCanvas;
use crate::component;
use crate::util::NeqAssign;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Canvas, Game, GameReq, GameState, Player};
use ferrogallic_shared::config::{CANVAS_HEIGHT, CANVAS_WIDTH, GUESS_SECONDS};
use ferrogallic_shared::domain::{
    Color, Epoch, Guess, Lobby, Lowercase, Nickname, Tool, U12Pair, UserId,
};
use gloo::events::{EventListener, EventListenerOptions};
use std::collections::BTreeMap;
use std::sync::Arc;
use time::Duration;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement, KeyboardEvent};
use yew::services::render::{RenderService, RenderTask};
use yew::services::websocket::{WebSocketService, WebSocketStatus};
use yew::utils::window;
use yew::{
    html, Callback, Component, ComponentLink, Html, NodeRef, PointerEvent, Properties, ShouldRender,
};

pub enum Msg {
    ConnStatus(WebSocketStatus),
    Message(Game),
    RemovePlayer(UserId, Epoch<UserId>),
    ChooseWord(Lowercase),
    SendGuess(Lowercase),
    Pointer(PointerAction),
    Undo,
    Render,
    SetTool(Tool),
    SetColor(Color),
    SetGlobalError(Error),
    Ignore,
}

pub enum PointerAction {
    Down(U12Pair),
    Move(U12Pair),
    Up(U12Pair),
}

#[derive(Clone, Properties)]
pub struct Props {
    pub app_link: ComponentLink<app::App>,
    pub lobby: Lobby,
    pub nick: Nickname,
}

pub struct InGame {
    link: ComponentLink<Self>,
    app_link: ComponentLink<app::App>,
    ws_service: WebSocketService,
    render_service: RenderService,
    lobby: Lobby,
    nick: Nickname,
    active_ws: Option<WebSocketApiTask<Game>>,
    scheduled_render: Option<RenderTask>,
    canvas_ref: NodeRef,
    canvas: Option<CanvasState>,
    pointer: PointerState,
    tool: Tool,
    color: Color,
    players: Arc<BTreeMap<UserId, Player>>,
    game: Arc<GameState>,
    guesses: Arc<Vec<Guess>>,
}

struct CanvasState {
    vr: VirtualCanvas,
    context: CanvasRenderingContext2d,
    _disable_touchstart: EventListener,
    _hook_ctrl_z: EventListener,
}

#[derive(Copy, Clone)]
enum PointerState {
    Up,
    Down { at: U12Pair },
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
            nick: props.nick,
            active_ws: None,
            scheduled_render: None,
            canvas_ref: Default::default(),
            canvas: None,
            pointer: PointerState::Up,
            tool: Default::default(),
            color: Default::default(),
            players: Default::default(),
            game: Default::default(),
            guesses: Default::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ConnStatus(status) => match status {
                WebSocketStatus::Opened => {
                    if let Some(ws) = &mut self.active_ws {
                        ws.send_api(&GameReq::Join(self.lobby.clone(), self.nick.clone()));
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
                Game::Canvas(event) => {
                    self.render_to_virtual(event);
                    self.schedule_render_to_canvas();
                    false
                }
                Game::CanvasBulk(events) => {
                    for event in events {
                        self.render_to_virtual(event);
                    }
                    self.schedule_render_to_canvas();
                    false
                }
                Game::Players(players) => {
                    self.players = players;
                    true
                }
                Game::Game(game) => {
                    self.game = game;
                    true
                }
                Game::Guess(guess) => {
                    Arc::make_mut(&mut self.guesses).push(guess);
                    true
                }
                Game::GuessBulk(guesses) => {
                    Arc::make_mut(&mut self.guesses).extend(guesses);
                    true
                }
                Game::Heartbeat => false,
            },
            Msg::RemovePlayer(user_id, epoch) => {
                if let Some(ws) = &mut self.active_ws {
                    ws.send_api(&GameReq::Remove(user_id, epoch));
                }
                false
            }
            Msg::ChooseWord(word) => {
                if let Some(ws) = &mut self.active_ws {
                    ws.send_api(&GameReq::Choose(word));
                }
                false
            }
            Msg::SendGuess(guess) => {
                if let Some(ws) = &mut self.active_ws {
                    ws.send_api(&GameReq::Guess(guess));
                }
                false
            }
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
                    self.schedule_render_to_canvas();
                    if let Some(ws) = &mut self.active_ws {
                        ws.send_api(&GameReq::Canvas(event));
                    }
                }
                false
            }
            Msg::Undo => {
                let event = Canvas::PopUndo;
                self.render_to_virtual(event);
                self.schedule_render_to_canvas();
                if let Some(ws) = &mut self.active_ws {
                    ws.send_api(&GameReq::Canvas(event));
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
            Msg::Ignore => false,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        let Props {
            app_link,
            lobby,
            nick,
        } = props;
        self.app_link = app_link;
        self.lobby.neq_assign(lobby) | self.nick.neq_assign(nick)
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            if let Some(canvas) = self.canvas_ref.cast::<HtmlCanvasElement>() {
                if let Some(context) = canvas
                    .get_context("2d")
                    .ok()
                    .flatten()
                    .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
                {
                    let disable_touchstart = EventListener::new_with_options(
                        &canvas.into(),
                        "touchstart",
                        EventListenerOptions::enable_prevent_default(),
                        |e| e.prevent_default(),
                    );
                    let hook_ctrl_z = EventListener::new_with_options(
                        &window().into(),
                        "keydown",
                        EventListenerOptions::enable_prevent_default(),
                        {
                            let link = self.link.clone();
                            move |e| {
                                let e = KeyboardEvent::from(JsValue::from(e));
                                let z = 90;
                                if e.ctrl_key() && e.key_code() == z {
                                    e.prevent_default();
                                    link.send_message(Msg::Undo);
                                }
                            }
                        },
                    );
                    self.canvas = Some(CanvasState {
                        vr: VirtualCanvas::new(),
                        context,
                        _disable_touchstart: disable_touchstart,
                        _hook_ctrl_z: hook_ctrl_z,
                    });
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
        }
    }

    fn view(&self) -> Html {
        let on_pointer_down =
            self.handle_pointer_event_if(|e| e.buttons() == 1, PointerAction::Down);
        let on_pointer_move = self.handle_pointer_event(PointerAction::Move);
        let on_pointer_up = self.handle_pointer_event(PointerAction::Up);

        let mut can_draw = false;
        let mut choose_words = None;
        let mut started = None;
        let mut guess_template = None;
        let _: () = match self.game.as_ref() {
            GameState::WaitingToStart { .. } => {
                can_draw = true;
            }
            GameState::ChoosingWords { choosing, words } => {
                if *choosing == self.nick.user_id() {
                    choose_words = Some(words.clone());
                }
            }
            GameState::Drawing {
                drawing,
                correct_scores: _,
                word,
                epoch: _,
                started: game_started,
                timed_out: _,
            } => {
                started = Some(*game_started);
                if *drawing == self.nick.user_id() {
                    can_draw = true;
                    guess_template = Some(component::guess_input::Template::reveal_all(&word));
                } else {
                    guess_template = Some(component::guess_input::Template::reveal_spaces(&word));
                }
            }
        };

        html! {
            <main class="window" style="max-width: 1500px; margin: auto">
                <div class="title-bar">
                    <div class="title-bar-text">
                        {"In Game - "}{&self.lobby}{" "}
                        {started.map(|started| html! {
                            <>{"("}<component::Timer started=started count_down_from=Duration::seconds(i64::from(GUESS_SECONDS))/>{")"}</>
                         }).unwrap_or_default()}
                    </div>
                </div>
                <article class="window-body" style="display: flex">
                    <section style="flex: 1; height: 804px">
                        <component::Players game_link=self.link.clone() players=self.players.clone()/>
                    </section>
                    <section style="margin: 0 8px; position: relative">
                        <fieldset style="padding-block-start: 2px; padding-block-end: 0px; padding-inline-start: 2px; padding-inline-end: 2px;">
                            <canvas
                                ref=self.canvas_ref.clone()
                                style=if can_draw { "" } else { "pointer-events: none" }
                                onpointerdown=on_pointer_down
                                onpointermove=on_pointer_move
                                onpointerup=&on_pointer_up
                                onpointerleave=on_pointer_up
                                width=CANVAS_WIDTH
                                height=CANVAS_HEIGHT
                            />
                        </fieldset>
                        <div style="position: relative">
                            <component::ColorToolbar game_link=self.link.clone() color=self.color/>
                            <component::ToolToolbar game_link=self.link.clone() tool=self.tool/>
                            <div
                                class="hatched-background"
                                style=if can_draw { "" } else { "position: absolute; top: 0; width: 100%; height: 100%" }
                            />
                        </div>
                        {match choose_words {
                            Some(words) => html! {
                                <component::ChoosePopup game_link=self.link.clone(), words=words />
                            },
                            None => html! {},
                        }}
                    </section>
                    <section style="flex: 1; height: 804px; display: flex; flex-direction: column">
                        <div style="flex: 1; min-height: 0">
                            <component::GuessArea players=self.players.clone() guesses=self.guesses.clone()/>
                        </div>
                        <component::GuessInput game_link=self.link.clone(), guess_template=guess_template/>
                    </section>
                </article>
            </main>
        }
    }
}

impl InGame {
    fn handle_pointer_event(
        &self,
        f: impl Fn(U12Pair) -> PointerAction + 'static,
    ) -> Callback<PointerEvent> {
        self.handle_pointer_event_if(|_| true, f)
    }

    fn handle_pointer_event_if(
        &self,
        pred: impl Fn(&PointerEvent) -> bool + 'static,
        f: impl Fn(U12Pair) -> PointerAction + 'static,
    ) -> Callback<PointerEvent> {
        self.link.callback(move |e: PointerEvent| {
            if pred(&e) {
                match e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
                    Some(target) => {
                        e.prevent_default();
                        let origin = target.get_bounding_client_rect();
                        Msg::Pointer(f(U12Pair::new(
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
        if let Some(canvas) = &mut self.canvas {
            canvas.vr.handle_event(event);
        }
    }

    fn schedule_render_to_canvas(&mut self) {
        if let scheduled @ None = &mut self.scheduled_render {
            *scheduled = Some(
                self.render_service
                    .request_animation_frame(self.link.callback(|_| Msg::Render)),
            );
        }
    }

    fn render_to_canvas(&mut self) {
        self.scheduled_render = None;
        if let Some(canvas) = &mut self.canvas {
            if let Err(e) = canvas.vr.render_to(&canvas.context) {
                log::warn!("Failed to render to canvas: {:?}", e);
            }
        }
    }
}
