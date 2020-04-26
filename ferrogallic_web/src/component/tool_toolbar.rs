use crate::page;
use crate::util::NeqAssign;
use ferrogallic_shared::domain::Tool;
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
        Tool::ALL
            .iter()
            .map(|&tool| {
                let onclick = self
                    .game_link
                    .callback(move |_| page::in_game::Msg::SetTool(tool));
                let class = if tool == self.tool { "selected" } else { "" };
                let text = match tool {
                    Tool::Pen(width) => width.text(),
                    Tool::Fill => "fill",
                };
                html! {
                    <button onclick=onclick class=class>
                        {text}
                    </button>
                }
            })
            .collect()
    }
}
