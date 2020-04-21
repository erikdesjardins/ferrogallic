use ferrogallic_shared::domain::{Lobby, Nickname};
use yew_router::Switch;

#[derive(Clone, Switch)]
pub enum AppRoute {
    #[to = "/join/{lobby}/as/{nickname}"]
    InGame { lobby: Lobby, nickname: Nickname },
    #[to = "/join/{lobby}"]
    ChooseName { lobby: Lobby },
    #[to = "/create"]
    Create,
}
