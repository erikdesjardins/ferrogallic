#![recursion_limit = "256"]

use wasm_bindgen::prelude::wasm_bindgen;

mod api;
mod component;
mod route;
mod util;

#[global_allocator]
// todo does this actually help?
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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

    yew::start_app::<component::App>()
}
