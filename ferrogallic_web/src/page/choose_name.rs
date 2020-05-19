use crate::route::AppRoute;
use crate::util::NeqAssign;
use ferrogallic_shared::domain::{Lobby, Nickname};
use yew::agent::{Dispatched, Dispatcher};
use yew::{html, Component, ComponentLink, Event, Html, InputData, Properties, ShouldRender};
use yew_router::agent::{RouteAgent, RouteRequest};
use yew_router::route::Route;

pub enum Msg {
    SetNick(Nickname),
    GoToLobby,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub lobby: Lobby,
}

pub struct ChooseName {
    link: ComponentLink<Self>,
    router: Dispatcher<RouteAgent>,
    props: Props,
    nick: Nickname,
}

impl Component for ChooseName {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            router: RouteAgent::dispatcher(),
            props,
            nick: Nickname::new(String::new()),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetNick(nick) => {
                self.nick = nick;
                true
            }
            Msg::GoToLobby => {
                self.router
                    .send(RouteRequest::ChangeRoute(Route::from(AppRoute::InGame {
                        lobby: self.props.lobby.clone(),
                        nick: self.nick.clone(),
                    })));
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let on_change_nick = self
            .link
            .callback(|e: InputData| Msg::SetNick(Nickname::new(e.value)));
        let on_join_game = self.link.callback(|e: Event| {
            e.prevent_default();
            Msg::GoToLobby
        });
        html! {
            <div style="display: flex; justify-content: space-evenly; align-items: flex-start">
                <main class="window" style="min-width: 300px">
                    <div class="title-bar">
                        <div class="title-bar-text">{"Join Game - "}{&self.props.lobby}</div>
                    </div>
                    <article class="window-body">
                        <form onsubmit=on_join_game>
                            <p class="field-row-stacked">
                                <label for="nickname">{"Nickname"}</label>
                                <input
                                    id="nickname"
                                    type="text"
                                    oninput=on_change_nick
                                    value=&self.nick
                                />
                            </p>
                            <section class="field-row" style="justify-content: flex-end">
                                <button disabled=self.nick.is_empty()>
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
