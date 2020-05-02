use ferrogallic_shared::domain::{Lobby, Nickname};
use yew_router::Switch;

#[derive(Clone, Switch)]
pub enum AppRoute {
    #[to = "/join/{lobby}/as/{nick}"]
    InGame { lobby: Lobby, nick: Nickname },
    #[to = "/join/{lobby}"]
    ChooseName { lobby: Lobby },
    #[to = "/create"]
    Create,
}
