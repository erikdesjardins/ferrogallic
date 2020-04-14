use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use crate::util::NeqAssign;

pub enum Msg {}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub lobby: String,
    pub nickname: String,
}

pub struct InGame {
    props: Props,
}

impl Component for InGame {
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
        html! {
            <p>{"In game page"}</p>
        }
    }
}
