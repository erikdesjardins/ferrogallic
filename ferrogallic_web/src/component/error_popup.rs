use crate::app;
use crate::util::ArcPtrEq;
use anyhow::Error;
use gloo::utils::window;
use yew::{html, Callback, Component, Context, Html, Properties};

pub enum Msg {}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub app_link: Callback<app::Msg>,
    pub error: Option<ArcPtrEq<Error>>,
}

pub struct ErrorPopup {}

impl Component for ErrorPopup {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let e = match &ctx.props().error {
            Some(e) => e,
            None => return html! {},
        };

        let message = e
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; caused by: ");
        let on_close = ctx.props().app_link.reform(|_| app::Msg::ClearError);
        let on_refresh = Callback::from(|_| {
            if let Err(e) = window().location().reload() {
                log::error!("Failed to reload: {:?}", e);
            }
        });

        html! {
            <dialog open=true class="hatched-background">
                <div class="window" style="min-width: 300px">
                    <div class="title-bar">
                        <div class="title-bar-text">{"Error!"}</div>
                        <div class="title-bar-controls">
                            <button aria-label="Close" onclick={on_close}/>
                        </div>
                    </div>
                    <div class="window-body">
                        <p>
                            <pre>{message}</pre>
                        </p>
                        <section class="field-row" style="justify-content: flex-end">
                            <button onclick={on_refresh}>{"Refresh"}</button>
                        </section>
                    </div>
                </div>
            </dialog>
        }
    }
}
