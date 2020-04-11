use std::convert::Infallible;

use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use crate::util::NeqAssign;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub lobby: String,
}

pub struct ChooseName {
    props: Props,
}

impl Component for ChooseName {
    type Message = Infallible;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        html! {
            <p>{"Choose name page"}</p>
        }
    }
}
