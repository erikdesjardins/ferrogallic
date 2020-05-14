use crate::page;
use crate::util::NeqAssign;
use boolinator::Boolinator;
use ferrogallic_shared::domain::{LineWidth, Tool};
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {}

#[derive(Clone, Properties)]
pub struct Props {
    pub game_link: ComponentLink<page::InGame>,
    pub tool: Tool,
}

pub struct ToolToolbar {
    game_link: ComponentLink<page::InGame>,
    tool: Tool,
}

impl Component for ToolToolbar {
    type Message = Msg;
    type Properties = Props;

    fn create(Props { game_link, tool }: Self::Properties, _: ComponentLink<Self>) -> Self {
        Self { game_link, tool }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, Props { game_link, tool }: Self::Properties) -> ShouldRender {
        self.game_link = game_link;
        self.tool.neq_assign(tool)
    }

    fn view(&self) -> Html {
        let tools = Tool::ALL
            .iter()
            .map(|&tool| {
                let onclick = self
                    .game_link
                    .callback(move |_| page::in_game::Msg::SetTool(tool));
                let active = (tool == self.tool).as_some("active");
                let (text, style) = match tool {
                    Tool::Pen(width) => (
                        "⚫",
                        match width {
                            LineWidth::Small => "font-size: 2px",
                            LineWidth::Normal => "font-size: 4px",
                            LineWidth::Medium => "font-size: 6px",
                            LineWidth::Large => "font-size: 10px",
                            LineWidth::Extra => "font-size: 14px",
                        },
                    ),
                    Tool::Fill => ("▧", "font-size: 28px"),
                };
                html! {
                    <button class=("tool-button", active) onclick=onclick style=style>
                        {text}
                    </button>
                }
            })
            .collect::<Html>();

        let on_undo = self.game_link.callback(|_| page::in_game::Msg::Undo);
        let undo = html! {
            <button class="tool-button" onclick=on_undo style="font-size: 28px">
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
