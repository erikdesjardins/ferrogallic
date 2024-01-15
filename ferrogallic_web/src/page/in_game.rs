use crate::api::{connect_api, TypedWebSocketWriter};
use crate::app;
use crate::audio::AudioService;
use crate::canvas::VirtualCanvas;
use crate::component;
use anyhow::{anyhow, Error};
use ferrogallic_shared::api::game::{Canvas, Game, GamePhase, GameReq, GameState, Player};
use ferrogallic_shared::config::{CANVAS_HEIGHT, CANVAS_WIDTH};
use ferrogallic_shared::domain::{
    Color, Epoch, Guess, I12Pair, LineWidth, Lobby, Lowercase, Nickname, Tool, UserId,
};
use gloo::events::{EventListener, EventListenerOptions};
use gloo::render::{request_animation_frame, AnimationFrame};
use std::collections::BTreeMap;
use std::convert::identity;
use std::mem;
use std::sync::Arc;
use time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, KeyboardEvent};
use yew::{html, Callback, Component, Context, Html, NodeRef, PointerEvent, Properties};

pub enum Msg {
    WebSocketConnected(TypedWebSocketWriter<Game>),
    WebSocketError(Error),
    Message(Game),
    RemovePlayer(UserId, Epoch<UserId>),
    ChooseWord(Lowercase),
    Pointer(PointerAction),
    Undo,
    Render,
    SetGuess(Lowercase),
    SendGuess,
    SetTool(Tool),
    SetColor(Color),
    Ignore,
}

pub enum PointerAction {
    Down(I12Pair),
    Move(I12Pair),
    Up(I12Pair),
}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub app_link: Callback<app::Msg>,
    pub lobby: Lobby,
    pub nick: Nickname,
}

pub struct InGame {
    link: Callback<Msg>,
    user_id: UserId,
    active_ws: Option<TypedWebSocketWriter<Game>>,
    audio: AudioService,
    scheduled_render: Option<AnimationFrame>,
    canvas_ref: NodeRef,
    canvas: Option<CanvasState>,
    pointer: PointerState,
    guess: Lowercase,
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
}

#[derive(Copy, Clone)]
enum PointerState {
    Up,
    Down { at: I12Pair },
}

impl Component for InGame {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            link: ctx.link().callback(identity),
            user_id: ctx.props().nick.user_id(),
            active_ws: None,
            audio: AudioService::new(),
            scheduled_render: None,
            canvas_ref: Default::default(),
            canvas: None,
            pointer: PointerState::Up,
            guess: Default::default(),
            tool: Default::default(),
            color: Default::default(),
            players: Default::default(),
            game: Default::default(),
            guesses: Default::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::WebSocketConnected(writer) => {
                self.active_ws = Some(writer);
                false
            }
            Msg::WebSocketError(e) => {
                self.active_ws = None;
                ctx.props().app_link.emit(app::Msg::SetError(e));
                false
            }
            Msg::Message(msg) => match msg {
                Game::Canvas(event) => {
                    self.render_to_virtual(event);
                    self.schedule_render_to_canvas(ctx);
                    false
                }
                Game::CanvasBulk(events) => {
                    for event in events {
                        self.render_to_virtual(event);
                    }
                    self.schedule_render_to_canvas(ctx);
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
                    self.play_sound(&guess);
                    Arc::make_mut(&mut self.guesses).push(guess);
                    true
                }
                Game::GuessBulk(guesses) => {
                    Arc::make_mut(&mut self.guesses).extend(guesses);
                    true
                }
                Game::ClearGuesses => {
                    self.guesses = Default::default();
                    true
                }
                Game::Heartbeat => false,
            },
            Msg::RemovePlayer(user_id, epoch) => {
                self.send_if_connected(ctx, &GameReq::Remove(user_id, epoch));
                false
            }
            Msg::ChooseWord(word) => {
                self.send_if_connected(ctx, &GameReq::Choose(word));
                false
            }
            Msg::SendGuess => {
                let guess = mem::take(&mut self.guess);
                self.send_if_connected(ctx, &GameReq::Guess(guess));
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
                    self.schedule_render_to_canvas(ctx);
                    self.send_if_connected(ctx, &GameReq::Canvas(event));
                }
                false
            }
            Msg::Undo => {
                let event = Canvas::PopUndo;
                self.render_to_virtual(event);
                self.schedule_render_to_canvas(ctx);
                self.send_if_connected(ctx, &GameReq::Canvas(event));
                false
            }
            Msg::Render => {
                self.render_to_canvas();
                false
            }
            Msg::SetGuess(guess) => {
                self.guess = guess;
                true
            }
            Msg::SetTool(tool) => {
                self.tool = tool;
                true
            }
            Msg::SetColor(color) => {
                self.color = color;
                true
            }
            Msg::Ignore => false,
        }
    }

    fn changed(&mut self, _ctx: &Context<Self>, props: &Self::Properties) -> bool {
        self.user_id = props.nick.user_id();
        true
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
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
                    self.canvas = Some(CanvasState {
                        vr: VirtualCanvas::new(),
                        context,
                        _disable_touchstart: disable_touchstart,
                    });
                }
            }

            self.connect_to_websocket(ctx);
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        enum Status<'a> {
            Waiting,
            Choosing(&'a Player),
            Drawing(&'a Player),
        }

        let mut can_draw = false;
        let mut choose_words = None;
        let mut cur_round = None;
        let mut status = Status::Waiting;
        let mut drawing_started = None;
        let mut guess_template = None;
        let _: () = match &self.game.phase {
            GamePhase::WaitingToStart => {
                can_draw = true;
            }
            GamePhase::ChoosingWords {
                round,
                choosing,
                words,
            } => {
                cur_round = Some(*round);
                if let Some(player) = self.players.get(choosing) {
                    status = Status::Choosing(player);
                }
                if *choosing == self.user_id {
                    choose_words = Some(words.clone());
                }
            }
            GamePhase::Drawing {
                round,
                drawing,
                correct: _,
                word,
                epoch: _,
                started,
            } => {
                cur_round = Some(*round);
                if let Some(player) = self.players.get(drawing) {
                    status = Status::Drawing(player);
                }
                drawing_started = Some(*started);
                if *drawing == self.user_id {
                    can_draw = true;
                    guess_template = Some((word.clone(), component::guess_template::Reveal::All));
                } else {
                    guess_template =
                        Some((word.clone(), component::guess_template::Reveal::Spaces));
                }
            }
        };

        let on_keydown;
        let on_pointerdown;
        let on_pointermove;
        let on_pointerup;
        if can_draw {
            on_keydown = ctx.link().callback(|e: KeyboardEvent| {
                let ctrl = e.ctrl_key();
                let msg = match e.key_code() {
                    49 /* 1 */ if !ctrl => Msg::SetTool(Tool::Pen(LineWidth::R0)),
                    50 /* 2 */ if !ctrl => Msg::SetTool(Tool::Pen(LineWidth::R1)),
                    51 /* 3 */ if !ctrl => Msg::SetTool(Tool::Pen(LineWidth::R2)),
                    52 /* 4 */ if !ctrl => Msg::SetTool(Tool::Pen(LineWidth::R4)),
                    53 /* 5 */ if !ctrl => Msg::SetTool(Tool::Pen(LineWidth::R7)),
                    70 /* f */ if !ctrl => Msg::SetTool(Tool::Fill),
                    90 /* z */ if ctrl => Msg::Undo,
                    _ => return Msg::Ignore,
                };
                e.prevent_default();
                msg
            });
            on_pointerdown = self.handle_pointer_event_if(
                ctx,
                |e| e.buttons() == 1,
                |e, target, at| {
                    if let Err(e) = target.focus() {
                        log::warn!("Failed to focus canvas: {:?}", e);
                    }
                    if let Err(e) = target.set_pointer_capture(e.pointer_id()) {
                        log::warn!("Failed to set pointer capture: {:?}", e);
                    }
                    PointerAction::Down(at)
                },
            );
            on_pointermove = self.handle_pointer_event(ctx, |_, _, at| PointerAction::Move(at));
            on_pointerup = self.handle_pointer_event(ctx, |e, target, at| {
                if let Err(e) = target.release_pointer_capture(e.pointer_id()) {
                    log::warn!("Failed to release pointer capture: {:?}", e);
                }
                PointerAction::Up(at)
            });
        } else {
            on_keydown = Callback::from(|_| {});
            let noop = Callback::from(|_| {});
            on_pointerdown = noop.clone();
            on_pointermove = noop.clone();
            on_pointerup = noop;
        }

        html! {
            <main class="window" style="max-width: 1500px; margin: auto">
                <div class="title-bar">
                    <div class="title-bar-text">{"In Game - "}{&ctx.props().lobby}</div>
                </div>
                <article class="window-body" style="display: flex">
                    <section style="flex: 1; height: 804px">
                        <component::Players game_link={self.link.clone()} players={self.players.clone()}/>
                    </section>
                    <section style="margin: 0 8px; position: relative" onkeydown={on_keydown}>
                        <fieldset style="padding-block-start: 2px; padding-block-end: 0px; padding-inline-start: 2px; padding-inline-end: 2px;">
                            <canvas
                                ref={self.canvas_ref.clone()}
                                style={"outline: initial" /* disable focus outline */}
                                tabindex={"-1" /* allow focus */}
                                onpointerdown={on_pointerdown}
                                onpointermove={on_pointermove}
                                onpointerup={on_pointerup}
                                width={CANVAS_WIDTH.to_string()}
                                height={CANVAS_HEIGHT.to_string()}
                            />
                        </fieldset>
                        <div style="position: relative">
                            <component::ColorToolbar game_link={self.link.clone()} color={self.color}/>
                            <component::ToolToolbar game_link={self.link.clone()} tool={self.tool}/>
                            <div
                                class="hatched-background"
                                style={if can_draw { "" } else { "position: absolute; top: 0; width: 100%; height: 100%" }}
                            />
                        </div>
                        {choose_words.map(|words| html! {
                            <component::ChoosePopup game_link={self.link.clone()} words={words} />
                        }).unwrap_or_default()}
                    </section>
                    <section style="flex: 1; height: 804px; display: flex; flex-direction: column">
                        <div style="flex: 1; min-height: 0; margin-bottom: 8px">
                            <component::GuessArea players={self.players.clone()} guesses={self.guesses.clone()}/>
                        </div>
                        <component::GuessInput game_link={self.link.clone()} guess={self.guess.clone()}/>
                    </section>
                </article>
                <footer class="status-bar">
                    <div>
                        {match status {
                            Status::Waiting => html! { {"Waiting to start"} },
                            Status::Choosing(player) => html! { <>{&player.nick}{" is choosing a word"}</> },
                            Status::Drawing(player) => html! { <>{&player.nick}{" is drawing"}</> },
                        }}
                    </div>
                    <div>
                        {drawing_started.map(|drawing_started| html! {
                            <component::Timer started={drawing_started} count_down_from={Duration::seconds(i64::from(self.game.config.guess_seconds))}/>
                         }).unwrap_or_default()}
                         {"/"}{self.game.config.guess_seconds}{" seconds"}
                    </div>
                    <div>
                        {cur_round.map(|cur_round| html! {
                            {cur_round}
                        }).unwrap_or_default()}
                        {"/"}{self.game.config.rounds}{" rounds"}
                    </div>
                    <div style="width: calc((min(100vw - 16px, 1500px) - 804px) / 2 - 6px)">
                        {guess_template.map(|(word, reveal)| html! {
                            <component::GuessTemplate word={word} reveal={reveal} guess={self.guess.clone()}/>
                        }).unwrap_or_default()}
                    </div>
                </footer>
            </main>
        }
    }
}

impl InGame {
    fn connect_to_websocket(&self, ctx: &Context<Self>) {
        match connect_api() {
            Ok((mut reader, mut writer)) => {
                let link = ctx.link().clone();
                let join_rec = GameReq::Join(ctx.props().lobby.clone(), ctx.props().nick.clone());
                spawn_local(async move {
                    match writer.wait_for_connection_and_send(&join_rec).await {
                        Ok(()) => link.send_message(Msg::WebSocketConnected(writer)),
                        Err(e) => link.send_message(Msg::WebSocketError(e)),
                    }
                });
                let link = ctx.link().clone();
                spawn_local(async move {
                    loop {
                        match reader.next_api().await {
                            Some(Ok(msg)) => {
                                link.send_message(Msg::Message(msg));
                            }
                            Some(Err(e)) => {
                                link.send_message(Msg::WebSocketError(e));
                                break;
                            }
                            None => {
                                link.send_message(Msg::WebSocketError(anyhow!("Websocket closed")));
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => ctx.link().send_message(Msg::WebSocketError(e)),
        }
    }

    fn send_if_connected(&mut self, ctx: &Context<Self>, req: &GameReq) {
        if let Some(ws) = &mut self.active_ws {
            if let Err(e) = ws.send_sync(req) {
                ctx.link().send_message(Msg::WebSocketError(e));
            }
        }
    }

    fn handle_pointer_event(
        &self,
        ctx: &Context<Self>,
        f: impl Fn(&PointerEvent, &HtmlElement, I12Pair) -> PointerAction + 'static,
    ) -> Callback<PointerEvent> {
        self.handle_pointer_event_if(ctx, |_| true, f)
    }

    fn handle_pointer_event_if(
        &self,
        ctx: &Context<Self>,
        pred: impl Fn(&PointerEvent) -> bool + 'static,
        f: impl Fn(&PointerEvent, &HtmlElement, I12Pair) -> PointerAction + 'static,
    ) -> Callback<PointerEvent> {
        ctx.link().callback(move |e: PointerEvent| {
            if pred(&e) {
                if let Some(target) = e.target().and_then(|t| t.dyn_into::<HtmlElement>().ok()) {
                    e.prevent_default();
                    let origin = target.get_bounding_client_rect();
                    Msg::Pointer(f(
                        &e,
                        &target,
                        I12Pair::new(
                            e.client_x() as i16 - origin.x() as i16,
                            e.client_y() as i16 - origin.y() as i16,
                        ),
                    ))
                } else {
                    Msg::Ignore
                }
            } else {
                Msg::Ignore
            }
        })
    }

    fn play_sound(&mut self, guess: &Guess) {
        if let Err(e) = self.audio.handle_guess(self.user_id, guess) {
            log::error!("Failed to play sound: {:?}", e);
        }
    }

    fn render_to_virtual(&mut self, event: Canvas) {
        if let Some(canvas) = &mut self.canvas {
            canvas.vr.handle_event(event);
        }
    }

    fn schedule_render_to_canvas(&mut self, ctx: &Context<Self>) {
        if let scheduled @ None = &mut self.scheduled_render {
            let link = ctx.link().clone();
            *scheduled = Some(request_animation_frame(move |_| {
                link.send_message(Msg::Render)
            }));
        }
    }

    fn render_to_canvas(&mut self) {
        self.scheduled_render = None;
        if let Some(canvas) = &mut self.canvas {
            if let Err(e) = canvas.vr.render_to(&canvas.context) {
                log::error!("Failed to render to canvas: {:?}", e);
            }
        }
    }
}
