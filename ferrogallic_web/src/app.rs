use crate::component;
use crate::page;
use crate::route::AppRoute;
use anyhow::Error;
use std::rc::Rc;
use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yew_router::router::Router;

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
        let app_link = self.link.clone();
        let render_app = Router::render(move |route| match route {
            AppRoute::Create => html! {<page::Create app_link=app_link.clone()/>},
            AppRoute::ChooseName { lobby } => html! {<page::ChooseName lobby=lobby/>},
            AppRoute::InGame { lobby, nickname } => {
                html! {<page::InGame app_link=app_link.clone() lobby=lobby nickname=nickname/>}
            }
        });
        let default_redirect = Router::redirect(|_| AppRoute::Create);
        html! {
            <>
                <main>
                    <Router<AppRoute> render=render_app redirect=default_redirect />
                </main>
                <component::ErrorBar error=self.error.clone() />
            </>
        }
    }
}
