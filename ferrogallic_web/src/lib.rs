#![recursion_limit = "1024"]

use wasm_bindgen::prelude::wasm_bindgen;

mod api;
mod app;
mod canvas;
mod component;
mod page;
mod route;
mod util;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    #[cfg(debug_assertions)]
    console_log::init_with_level(log::Level::Trace).expect("initializing logger");

    yew::start_app::<app::App>()
}
