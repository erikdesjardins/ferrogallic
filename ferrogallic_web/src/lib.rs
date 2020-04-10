use wasm_bindgen::prelude::wasm_bindgen;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    alert("testing testing 1 2 3");
}
