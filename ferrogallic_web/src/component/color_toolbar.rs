use crate::page;
use ferrogallic_shared::domain::Color;
use yew::{classes, html, Callback, Component, Context, Html, Properties};

pub enum Msg {}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub game_link: Callback<page::in_game::Msg>,
    pub color: Color,
}

pub struct ColorToolbar {}

impl Component for ColorToolbar {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let colors = Color::ALL
            .iter()
            .map(|&color| {
                let on_click = ctx.props()
                    .game_link
                    .reform(move |_| page::in_game::Msg::SetColor(color));
                let active = (color == ctx.props().color).then_some("active");
                let style = format!(
                    "background-color: rgb({}, {}, {})",
                    color.r, color.g, color.b
                );
                html! {
                    <button onclick={on_click} class={classes!("color-button", active)} style={style}/>
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
