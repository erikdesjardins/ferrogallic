use crate::route::AppRoute;
use crate::util::NeqAssign;
use ferrogallic_shared::domain::{Lobby, Nickname};
use yew::agent::{Dispatched, Dispatcher};
use yew::{html, Component, ComponentLink, Event, Html, InputData, Properties, ShouldRender};
use yew_router::agent::{RouteAgent, RouteRequest};
use yew_router::route::Route;

pub enum Msg {
    SetNickname(Nickname),
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
    nickname: Nickname,
}

impl Component for ChooseName {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            router: RouteAgent::dispatcher(),
            props,
            nickname: Nickname::new("".to_string()),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetNickname(nickname) => {
                self.nickname = nickname;
                true
            }
            Msg::GoToLobby => {
                self.router
                    .send(RouteRequest::ChangeRoute(Route::from(AppRoute::InGame {
                        lobby: self.props.lobby.clone(),
                        nickname: self.nickname.clone(),
                    })));
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let on_change_nickname = self
            .link
            .callback(|e: InputData| Msg::SetNickname(Nickname::new(e.value)));
        let on_join_game = self.link.callback(|e: Event| {
            e.prevent_default();
            Msg::GoToLobby
        });
        html! {
            <fieldset>
                <legend>{"Joining lobby: "}{&self.props.lobby}</legend>
                <form onsubmit=on_join_game>
                    <input
                        type="text"
                        placeholder="Nickname"
                        oninput=on_change_nickname
                        value=&self.nickname
                    />
                    <input
                        type="submit"
                        value="Join"
                        disabled=self.nickname.is_empty()
                    />
                </form>
            </fieldset>
        }
    }
}
