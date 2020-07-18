use crate::page;
use crate::util::NeqAssign;
use boolinator::Boolinator;
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
                let on_click = self
                    .game_link
                    .callback(move |_| page::in_game::Msg::SetColor(color));
                let active = (color == self.color).as_some("active");
                let style = format!(
                    "background-color: rgb({}, {}, {})",
                    color.r, color.g, color.b
                );
                html! {
                    <button onclick=on_click class=("color-button", active) style=style/>
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
