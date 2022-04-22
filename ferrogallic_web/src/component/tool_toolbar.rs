use crate::page;
use boolinator::Boolinator;
use ferrogallic_shared::domain::{LineWidth, Tool};
use yew::{classes, html, Callback, Component, Context, Html, Properties};

pub enum Msg {}

#[derive(PartialEq, Properties)]
pub struct Props {
    pub game_link: Callback<page::in_game::Msg>,
    pub tool: Tool,
}

pub struct ToolToolbar {}

impl Component for ToolToolbar {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let tools = Tool::ALL
            .iter()
            .map(|&tool| {
                let on_click = ctx
                    .props()
                    .game_link
                    .reform(move |_| page::in_game::Msg::SetTool(tool));
                let active = (tool == ctx.props().tool).as_some("active");
                let (text, style, title) = match tool {
                    Tool::Pen(width) => (
                        "⚫",
                        match width {
                            LineWidth::R0 => "font-size: 2px",
                            LineWidth::R1 => "font-size: 4px",
                            LineWidth::R2 => "font-size: 6px",
                            LineWidth::R4 => "font-size: 10px",
                            LineWidth::R7 => "font-size: 14px",
                        },
                        match width {
                            LineWidth::R0 => "Pen (1)",
                            LineWidth::R1 => "Pen (2)",
                            LineWidth::R2 => "Pen (3)",
                            LineWidth::R4 => "Pen (4)",
                            LineWidth::R7 => "Pen (5)",
                        },
                    ),
                    Tool::Fill => ("▧", "font-size: 28px", "Fill (F)"),
                };
                html! {
                    <button class={classes!("tool-button", active)} title={title} onclick={on_click} style={style}>
                        {text}
                    </button>
                }
            })
            .collect::<Html>();

        let on_undo = ctx.props().game_link.reform(|_| page::in_game::Msg::Undo);
        let undo = html! {
            <button class="tool-button" title="Undo (Ctrl-Z)" onclick={on_undo} style="font-size: 28px">
                {"↶"}
            </button>
        };

        html! {
            <div class="tool-buttons">
                {tools}
                {undo}
            </div>
        }
    }
}
