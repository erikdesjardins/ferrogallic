use crate::page;
use crate::util::NeqAssign;
use std::sync::Arc;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub words: Arc<[Arc<str>]>,
}

pub struct ChooseToolbar {
    game_link: ComponentLink<page::InGame>,
    words: Arc<[Arc<str>]>,
}

impl Component for ChooseToolbar {
    type Message = Msg;
    type Properties = Props;

    fn create(Props { game_link, words }: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { game_link, words }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, Props { game_link, words }: Self::Properties) -> ShouldRender {
        self.game_link = game_link;
        self.words.neq_assign(words)
    }

    fn view(&self) -> Html {
        self.words
            .iter()
            .map(|word| {
                let onclick = self.game_link.callback({
                    let word = word.clone();
                    move |_| page::in_game::Msg::ChooseWord(word.clone())
                });
                html! {
                    <button onclick=onclick>{word}</button>
                }
            })
            .collect()
    }
}
