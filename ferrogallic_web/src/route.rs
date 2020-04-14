use yew_router::Switch;

#[derive(Clone, Switch)]
pub enum AppRoute {
    #[to = "/join/{lobby}/as/{nickname}"]
    InGame { lobby: String, nickname: String },
    #[to = "/join/{lobby}"]
    ChooseName { lobby: String },
    #[to = "/create"]
    Create,
}
