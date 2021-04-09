use ferrogallic_shared::domain::{Guess, UserId};
use ferrogallic_shared::paths;
use js_sys::Promise;
use wasm_bindgen::JsValue;
use web_sys::HtmlAudioElement;

pub struct AudioService {
    elems: Result<Elems, JsValue>,
}

struct Elems {
    chimes: HtmlAudioElement,
    chord: HtmlAudioElement,
    ding: HtmlAudioElement,
    tada: HtmlAudioElement,
    asterisk: HtmlAudioElement,
    exclam: HtmlAudioElement,
    maximize: HtmlAudioElement,
    shutdown: HtmlAudioElement,
}

impl AudioService {
    pub fn new() -> Self {
        let elem =
            |path| HtmlAudioElement::new_with_src(&format!("/{}/{}", paths::audio::PREFIX, path));
        let elems = || {
            Ok(Elems {
                chimes: elem(paths::audio::CHIMES)?,
                chord: elem(paths::audio::CHORD)?,
                ding: elem(paths::audio::DING)?,
                tada: elem(paths::audio::TADA)?,
                asterisk: elem(paths::audio::ASTERISK)?,
                exclam: elem(paths::audio::EXCLAM)?,
                maximize: elem(paths::audio::MAXIMIZE)?,
                shutdown: elem(paths::audio::SHUTDOWN)?,
            })
        };

        Self { elems: elems() }
    }

    pub fn handle_guess(&mut self, user_id: UserId, guess: &Guess) -> Result<(), JsValue> {
        let elems = self.elems.as_ref()?;
        let elem = match guess {
            Guess::NowChoosing(uid) if *uid == user_id => &elems.maximize,
            Guess::NowDrawing(_) => &elems.exclam,
            Guess::Guess(_, _) => &elems.ding,
            Guess::CloseGuess(_) => &elems.asterisk,
            Guess::Correct(uid) if *uid == user_id => &elems.tada,
            Guess::Correct(_) => &elems.chimes,
            Guess::TimeExpired(_) => &elems.chord,
            Guess::GameOver => &elems.shutdown,
            _ => return Ok(()),
        };

        elem.set_current_time(0.);
        // ignore the promise, we don't care when it starts playing
        let _: Promise = elem.play()?;

        Ok(())
    }
}
