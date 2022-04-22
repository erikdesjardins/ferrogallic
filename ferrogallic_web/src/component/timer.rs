use gloo::timers::callback::Interval;
use js_sys::Date;
use time::Duration;
use time::OffsetDateTime;
use yew::{html, Component, Context, Html, Properties};

pub enum Msg {
    Tick,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub started: OffsetDateTime,
    pub count_down_from: Duration,
}

pub struct Timer {
    #[allow(dead_code)] // timer is cancelled when this is dropped
    active_timer: Interval,
}

impl Component for Timer {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let active_timer = Interval::new(1000, move || link.send_message(Msg::Tick));
        Self { active_timer }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => true,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let elapsed = Duration::milliseconds(
            Date::now() as i64 - ctx.props().started.unix_timestamp() * 1000,
        );
        let time_left = ctx.props().count_down_from - elapsed;
        let seconds_left = time_left.whole_seconds();
        html! {
            {seconds_left}
        }
    }
}
