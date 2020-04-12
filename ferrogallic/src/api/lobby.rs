use std::convert::Infallible;

use rand::seq::SliceRandom;
use rand::thread_rng;

use ferrogallic_api::RandomLobbyName;

use crate::words;

pub fn random_name(_: ()) -> Result<RandomLobbyName, Infallible> {
    let mut rng = thread_rng();
    let lobby = words::COMMON
        .choose_multiple(&mut rng, 3)
        .copied()
        .map(|word| word[0..1].to_uppercase() + &word[1..])
        .collect();
    Ok(RandomLobbyName { lobby })
}
