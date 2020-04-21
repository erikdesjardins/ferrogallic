use ferrogallic_shared::api::game::Canvas;
use ferrogallic_shared::config::{CANVAS_HEIGHT, CANVAS_WIDTH};
use wasm_bindgen::{Clamped, JsValue};
use web_sys::{CanvasRenderingContext2d, ImageData};

mod flood_fill;

pub trait CanvasRenderingContext2dExt {
    fn initialize(&self);

    fn handle_event(&self, event: Canvas);
}

impl CanvasRenderingContext2dExt for CanvasRenderingContext2d {
    fn initialize(&self) {
        self.set_line_cap("round");
        self.set_line_join("bevel");
    }

    fn handle_event(&self, event: Canvas) {
        match event {
            Canvas::LineStart { x, y, width, color } => {
                self.begin_path();
                self.set_line_width(width.px().into());
                self.set_stroke_style(&JsValue::from_str(color.css()));
                self.move_to(x.into(), y.into());
            }
            Canvas::LineTo { x, y } => {
                self.line_to(x.into(), y.into());
                self.stroke();
                self.begin_path();
                self.move_to(x.into(), y.into());
            }
            Canvas::Fill { x, y, color } => {
                if let Ok(image_data) =
                    self.get_image_data(0., 0., CANVAS_WIDTH.into(), CANVAS_HEIGHT.into())
                {
                    let Clamped(mut data) = image_data.data();
                    // todo flood_fill::fill(&mut data, x, y, color.argb());
                    // todo actually this is virtually free, since we just create a view into wasm memory
                    //  so let's store the buffer in rust, and RAF to call put_image_data
                    if let Ok(image_data) = ImageData::new_with_u8_clamped_array(
                        Clamped(&mut data),
                        CANVAS_WIDTH.into(),
                    ) {
                        let _ = self.put_image_data(&image_data, 0., 0.);
                    }
                }
            }
        }
    }
}
