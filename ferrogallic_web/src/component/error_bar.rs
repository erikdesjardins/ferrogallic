use crate::util::NeqAssign;
use anyhow::Error;
use std::rc::Rc;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub error: Option<Rc<Error>>,
}

pub struct ErrorBar {
    message: Option<String>,
}

impl Component for ErrorBar {
    type Message = Msg;
    type Properties = Props;

    fn create(Props { error }: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            message: Self::format_error(error),
        }
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {}
    }

    fn change(&mut self, Props { error }: Self::Properties) -> ShouldRender {
        let new_message = Self::format_error(error);
        self.message.neq_assign(new_message)
    }

    fn view(&self) -> Html {
        match &self.message {
            Some(msg) => {
                html! {
                    <p class="error-bar">{msg}</p>
                }
            }
            None => {
                html! {
                    <></>
                }
            }
        }
    }
}

impl ErrorBar {
    fn format_error(e: Option<Rc<Error>>) -> Option<String> {
        match e {
            Some(e) => Some(
                e.chain()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; caused by: "),
            ),
            None => None,
        }
    }
}
