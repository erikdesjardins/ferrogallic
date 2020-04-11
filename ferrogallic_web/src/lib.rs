use wasm_bindgen::prelude::wasm_bindgen;

mod component;
mod util;

#[global_allocator]
// todo does this actually help?
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    #[cfg(debug_assertions)]
    console_log::init().expect("initializing logger");

    yew::start_app::<component::App>()
}
