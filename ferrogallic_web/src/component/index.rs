use std::convert::Infallible;

use yew::{html, Component, ComponentLink, Html, ShouldRender};

pub struct Index;

impl Component for Index {
    type Message = Infallible;
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn view(&self) -> Html {
        html! {
            <p>{"Index page"}</p>
        }
    }
}
