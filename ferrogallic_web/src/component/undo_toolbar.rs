use crate::page;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
}

pub struct UndoToolbar {
    game_link: ComponentLink<page::InGame>,
}

impl Component for UndoToolbar {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self {
            game_link: props.game_link,
        }
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {}
    }

    fn change(&mut self, Props { game_link }: Self::Properties) -> ShouldRender {
        self.game_link = game_link;
        false
    }

    fn view(&self) -> Html {
        let onclick = self.game_link.callback(move |_| page::in_game::Msg::Undo);
        html! {
            <button onclick=onclick>
                {"\u{21b6}"}
            </button>
        }
    }
}
