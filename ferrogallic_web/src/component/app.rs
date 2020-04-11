use std::convert::Infallible;

use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yew_router::router::Router;
use yew_router::Switch;

use crate::component;

pub struct App;

#[derive(Clone, Switch)]
enum Route {
    #[to = "/join/{lobby}/as/{name}"]
    InGame { lobby: String, name: String },
    #[to = "/join/{lobby}"]
    ChooseName { lobby: String },
    #[to = "/"]
    Index,
}

impl Component for App {
    type Message = Infallible;
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn view(&self) -> Html {
        html! {
            <Router<Route, ()>
                render = Router::render(|switch: Route| {
                    match switch {
                        Route::Index => html!{<component::Index/>},
                        Route::ChooseName { lobby } => html!{<component::ChooseName lobby = lobby/>},
                        Route::InGame { lobby, name } => html!{<component::InGame lobby = lobby, name = name/>},
                    }
                })
            />
        }
    }
}
