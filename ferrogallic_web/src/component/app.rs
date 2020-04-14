use std::rc::Rc;

use anyhow::Error;
use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yew_router::router::Router;

use crate::component;
use crate::route::AppRoute;

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
        let render_app = Router::render(move |route| match route {
            AppRoute::Create => html! {<component::Create app_link=app_link.clone()/>},
            AppRoute::ChooseName { lobby } => html! {<component::ChooseName lobby=lobby/>},
            AppRoute::InGame { lobby, nickname } => {
                html! {<component::InGame lobby=lobby, nickname=nickname/>}
            }
        });
        let default_redirect = Router::redirect(|_| AppRoute::Create);
        html! {
            <>
                <style>{css}</style>
                <main>
                    <Router<AppRoute> render=render_app redirect=default_redirect />
                </main>
                <component::ErrorBar error=self.error.clone() />
            </>
        }
    }
}
