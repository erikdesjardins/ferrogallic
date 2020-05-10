use crate::component;
use crate::page;
use crate::route::AppRoute;
use anyhow::Error;
use std::sync::Arc;
use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yew_router::router::Router;

pub enum Msg {
    SetError(Error),
    ClearError,
}

pub struct App {
    link: ComponentLink<Self>,
    error: Option<Arc<Error>>,
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
                self.error = Some(Arc::new(e));
                true
            }
            Msg::ClearError => {
                self.error = None;
                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        let () = props;
        false
    }

    fn view(&self) -> Html {
        let app_link = self.link.clone();
        let render_app = Router::render(move |route| match route {
            AppRoute::Create => html! {<page::Create app_link=app_link.clone()/>},
            AppRoute::ChooseName { lobby } => html! {<page::ChooseName lobby=lobby/>},
            AppRoute::InGame { lobby, nick } => {
                html! {<page::InGame app_link=app_link.clone() lobby=lobby nick=nick/>}
            }
        });
        let default_redirect = Router::redirect(|_| AppRoute::Create);
        html! {
            <>
                <Router<AppRoute> render=render_app redirect=default_redirect />
                <component::ErrorPopup app_link=self.link.clone() error=self.error.clone() />
            </>
        }
    }
}
