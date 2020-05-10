use crate::page;
use crate::util::{NeqAssign, StrExt};
use ferrogallic_shared::domain::Color;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub color: Color,
}

pub struct ColorToolbar {
    game_link: ComponentLink<page::InGame>,
    color: Color,
}

impl Component for ColorToolbar {
    type Message = Msg;
    type Properties = Props;

    fn create(Props { game_link, color }: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { game_link, color }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, Props { game_link, color }: Self::Properties) -> ShouldRender {
        self.game_link = game_link;
        self.color.neq_assign(color)
    }

    fn view(&self) -> Html {
        let colors = Color::ALL
            .iter()
            .map(|&color| {
                let onclick = self
                    .game_link
                    .callback(move |_| page::in_game::Msg::SetColor(color));
                let active = "active".class_if(color == self.color);
                html! {
                    <button onclick=onclick class=("color-button", active) style=format!("background-color: {}", color.css())/>
                }
            })
            .collect::<Html>();

        html! {
            <div class="color-buttons">
                {colors}
            </div>
        }
    }
}
