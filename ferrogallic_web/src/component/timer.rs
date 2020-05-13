use crate::util::NeqAssign;
use js_sys::Date;
use time::Duration;
use time::OffsetDateTime;
use yew::services::interval::IntervalTask;
use yew::services::IntervalService;
use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

pub enum Msg {
    Tick,
}

#[derive(Clone, Properties, PartialEq, Eq)]
pub struct Props {
    pub started: OffsetDateTime,
    pub count_down_from: Duration,
}

pub struct Timer {
    link: ComponentLink<Timer>,
    interval_service: IntervalService,
    props: Props,
    active_timer: IntervalTask,
}

impl Component for Timer {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut interval_service = IntervalService::new();
        let active_timer = spawn_timer(&mut interval_service, &link);
        Self {
            link,
            interval_service,
            props,
            active_timer,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Tick => true,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props.neq_assign(props) {
            self.active_timer = spawn_timer(&mut self.interval_service, &self.link);
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        let elapsed =
            Duration::milliseconds(Date::now() as i64 - self.props.started.timestamp() * 1000);
        let time_left = self.props.count_down_from - elapsed;
        let seconds_left = time_left.whole_seconds();
        html! {
            <>{seconds_left}{"s"}</>
        }
    }
}

fn spawn_timer(
    interval_service: &mut IntervalService,
    link: &ComponentLink<Timer>,
) -> IntervalTask {
    interval_service.spawn(
        std::time::Duration::from_secs(1),
        link.callback(|()| Msg::Tick),
    )
}
