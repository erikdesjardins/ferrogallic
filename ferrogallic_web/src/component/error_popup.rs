use crate::app;
use anyhow::Error;
use std::sync::Arc;
use yew::utils::window;
use yew::{html, Callback, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub app_link: ComponentLink<app::App>,
    pub error: Option<Arc<Error>>,
}

pub struct ErrorPopup {
    app_link: ComponentLink<app::App>,
    error: Option<Arc<Error>>,
}

impl Component for ErrorPopup {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            app_link: props.app_link,
            error: props.error,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, Props { app_link, error }: Self::Properties) -> ShouldRender {
        self.app_link = app_link;
        let error_changed = match (&self.error, &error) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        self.error = error;
        error_changed
    }

    fn view(&self) -> Html {
        let e = match &self.error {
            Some(e) => e,
            None => return html! {},
        };

        let message = e
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; caused by: ");
        let on_close = self.app_link.callback(|_| app::Msg::ClearError);
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
                            <button aria-label="Close" onclick=on_close/>
                        </div>
                    </div>
                    <div class="window-body">
                        <p>
                            <pre>{message}</pre>
                        </p>
                        <section class="field-row" style="justify-content: flex-end">
                            <button onclick=on_refresh>{"Refresh"}</button>
                        </section>
                    </div>
                </div>
            </dialog>
        }
    }
}
