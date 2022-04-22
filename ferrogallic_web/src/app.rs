use crate::component;
use crate::page;
use crate::route::AppRoute;
use crate::util::ArcPtrEq;
use anyhow::Error;
use std::convert::identity;
use yew::{html, Callback, Component, Context, Html};
use yew_router::router::BrowserRouter;
use yew_router::Switch;

pub enum Msg {
    SetError(Error),
    ClearError,
}

pub struct App {
    link: Callback<Msg>,
    error: Option<ArcPtrEq<Error>>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            link: ctx.link().callback(identity),
            error: None,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetError(e) => {
                self.error = Some(e.into());
                true
            }
            Msg::ClearError => {
                self.error = None;
                true
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let app_link = self.link.clone();
        let render_app = Switch::render(move |route| match route {
            AppRoute::Create => html! {<page::Create app_link={app_link.clone()}/>},
            AppRoute::ChooseName { lobby } => html! {<page::ChooseName lobby={lobby.0.clone()}/>},
            AppRoute::InGame { lobby, nick } => {
                html! {<page::InGame app_link={app_link.clone()} lobby={lobby.0.clone()} nick={nick.0.clone()}/>}
            }
        });
        html! {
            <>
                <BrowserRouter>
                    <Switch<AppRoute> render={render_app}/>
                </BrowserRouter>
                <component::ErrorPopup app_link={self.link.clone()} error={self.error.clone()} />
            </>
        }
    }
}
