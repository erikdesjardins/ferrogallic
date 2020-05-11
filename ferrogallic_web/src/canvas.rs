use ferrogallic_shared::api::game::Canvas;
use ferrogallic_shared::domain::CanvasBuffer;
use std::mem;
use wasm_bindgen::{Clamped, JsValue};
use web_sys::{CanvasRenderingContext2d, ImageData};

mod draw;
mod flood_fill;

pub struct VirtualCanvas {
    buffer: Box<CanvasBuffer>,
    undo_stage: Option<Box<CanvasBuffer>>,
    undo_stack: Vec<Box<CanvasBuffer>>,
}

impl VirtualCanvas {
    pub fn new() -> Self {
        Self {
            buffer: CanvasBuffer::boxed(),
            undo_stage: Default::default(),
            undo_stack: Default::default(),
        }
    }

    pub fn handle_event(&mut self, event: Canvas) {
        match event {
            Canvas::Line {
                from,
                to,
                width,
                color,
            } => {
                draw::stroke_line(
                    &mut self.buffer,
                    from.x(),
                    from.y(),
                    to.x(),
                    to.y(),
                    width,
                    color,
                );
            }
            Canvas::Fill { at, color } => {
                flood_fill::fill(&mut self.buffer, at.x() as usize, at.y() as usize, color);
            }
            Canvas::PushUndo => {
                let buffer = self.buffer.clone_boxed();
                let prev_buffer = mem::replace(&mut self.undo_stage, Some(buffer));
                if let Some(prev) = prev_buffer {
                    self.undo_stack.push(prev);
                }
            }
            Canvas::PopUndo => {
                self.undo_stage = None;
                self.buffer = match self.undo_stack.pop() {
                    Some(undo) => undo,
                    // all the way back to the beginning
                    None => CanvasBuffer::boxed(),
                };
            }
            Canvas::Clear => {
                *self = Self::new();
            }
        }
    }

    pub fn render_to(&mut self, canvas: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        let width = self.buffer.x_len();
        let image_data = ImageData::new_with_u8_clamped_array(
            Clamped(self.buffer.as_mut_bytes()),
            width as u32,
        )?;
        canvas.put_image_data(&image_data, 0., 0.)
    }
}
