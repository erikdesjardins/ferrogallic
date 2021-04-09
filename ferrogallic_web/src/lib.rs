#![recursion_limit = "1024"]
#![allow(clippy::let_unit_value, clippy::match_bool)]

use wasm_bindgen::prelude::wasm_bindgen;

mod api;
mod app;
mod audio;
mod canvas;
mod component;
mod page;
mod route;
mod util;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    #[cfg(debug_assertions)]
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));

    yew::start_app::<app::App>()
}
