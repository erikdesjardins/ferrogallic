use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, InputEvent};

pub trait InputEventExt {
    fn target_value(&self) -> String;
}

impl InputEventExt for InputEvent {
    fn target_value(&self) -> String {
        let optional_value = || Some(self.target()?.dyn_into::<HtmlInputElement>().ok()?.value());
        optional_value().unwrap_or_default()
    }
}
