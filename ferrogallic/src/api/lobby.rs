use crate::words;
use ferrogallic_shared::api::RandomLobbyName;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::convert::Infallible;

pub fn random_name(_: ()) -> Result<RandomLobbyName, Infallible> {
    let mut rng = thread_rng();
    let lobby = words::COMMON_FOR_ROOM_NAMES
        .choose_multiple(&mut rng, 3)
        .map(|word| word[0..1].to_uppercase() + &word[1..])
        .collect();
    Ok(RandomLobbyName { lobby })
}
