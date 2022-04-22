use crate::page;
use crate::util::ArcPtrEq;
use ferrogallic_shared::domain::Lowercase;
use yew::{html, Callback, Component, Context, Html, Properties};

pub enum Msg {}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub game_link: Callback<page::in_game::Msg>,
    pub words: ArcPtrEq<[Lowercase]>,
}

pub struct ChoosePopup {}

impl Component for ChoosePopup {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let words = ctx
            .props()
            .words
            .iter()
            .map(|word| {
                let on_click = ctx.props().game_link.reform({
                    let word = word.clone();
                    move |_| page::in_game::Msg::ChooseWord(word.clone())
                });
                html! {
                    <button onclick={on_click}>{word}</button>
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
