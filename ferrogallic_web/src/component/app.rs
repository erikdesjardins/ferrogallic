use std::rc::Rc;

use anyhow::Error;
use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yew_router::router::Router;

use crate::component;
use crate::route::Route;

pub enum Msg {
    SetError(Error),
}

pub struct App {
    link: ComponentLink<Self>,
    error: Option<Rc<Error>>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self { link, error: None }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::SetError(e) => {
                self.error = Some(Rc::new(e));
                true
            }
        }
    }

    fn view(&self) -> Html {
        let css = include_str!("../styles/app.css");
        let app_link = self.link.clone();
        html! {
            <>
                <style>{css}</style>
                <main>
                    <Router<Route>
                        render = Router::render(move |route| {
                            match route {
                                Route::Create => html!{<component::Create app_link=app_link.clone()/>},
                                Route::ChooseName { lobby } => html!{<component::ChooseName lobby = lobby/>},
                                Route::InGame { lobby, name } => html!{<component::InGame lobby = lobby, name = name/>},
                            }
                        }),
                        redirect = Router::redirect(|_| Route::Create)
                    />
                </main>
                <component::ErrorBar error=self.error.clone() />
            </>
        }
    }
}
