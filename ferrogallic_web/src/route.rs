use yew_router::Switch;

#[derive(Clone, Switch)]
pub enum Route {
    #[to = "/join/{lobby}/as/{name}"]
    InGame { lobby: String, name: String },
    #[to = "/join/{lobby}"]
    ChooseName { lobby: String },
    #[to = "/create"]
    Create,
}
