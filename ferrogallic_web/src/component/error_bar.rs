use crate::util::NeqAssign;
use anyhow::Error;
use std::sync::Arc;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub error: Option<Arc<Error>>,
}

impl PartialEq for Props {
    fn eq(&self, Self { error }: &Self) -> bool {
        match (&self.error, error) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

pub struct ErrorBar {
    props: Props,
}

impl Component for ErrorBar {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        match &self.props.error {
            Some(e) => {
                let message = e
                    .chain()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; caused by: ");
                html! {
                    <p class="error-bar">{message}</p>
                }
            }
            None => {
                html! {}
            }
        }
    }
}
