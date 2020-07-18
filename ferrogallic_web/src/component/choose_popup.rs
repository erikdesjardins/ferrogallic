use crate::page;
use crate::util::NeqAssign;
use ferrogallic_shared::domain::Lowercase;
use std::sync::Arc;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub words: Arc<[Lowercase]>,
}

pub struct ChoosePopup {
    game_link: ComponentLink<page::InGame>,
    words: Arc<[Lowercase]>,
}

impl Component for ChoosePopup {
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
        let words = self
            .words
            .iter()
            .map(|word| {
                let on_click = self.game_link.callback({
                    let word = word.clone();
                    move |_| page::in_game::Msg::ChooseWord(word.clone())
                });
                html! {
                    <button onclick=on_click>{word}</button>
                }
            })
            .collect::<Html>();

        html! {
            <dialog open=true class="hatched-background">
                <div class="window">
                    <div class="title-bar">
                        <div class="title-bar-text">{"Choose Word"}</div>
                    </div>
                    <div class="window-body">
                        <section class="field-row" style="justify-content: flex-end">
                            {words}
                        </section>
                    </div>
                </div>
            </dialog>
        }
    }
}
