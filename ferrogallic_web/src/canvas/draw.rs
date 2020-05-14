use ferrogallic_shared::domain::{CanvasBuffer, Color, LineWidth};
use std::convert::TryFrom;

pub fn stroke_line(
    buf: &mut CanvasBuffer,
    x1: i16,
    y1: i16,
    x2: i16,
    y2: i16,
    width: LineWidth,
    color: Color,
) {
    each_point_in_line(
        i32::from(x1),
        i32::from(y1),
        i32::from(x2),
        i32::from(y2),
        |x, y| fill_circle(buf, x, y, width, color),
    );
}

pub fn each_point_in_line(mut x1: i32, mut y1: i32, x2: i32, y2: i32, mut f: impl FnMut(i32, i32)) {
    let dx = (x2 - x1).abs();
    let sx = (x2 - x1).signum();
    let dy = -(y2 - y1).abs();
    let sy = (y2 - y1).signum();
    let mut err = dx + dy;
    loop {
        f(x1, y1);
        if x1 == x2 && y1 == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x1 += sx;
        }
        if e2 <= dx {
            err += dx;
            y1 += sy;
        }
    }
}

pub fn fill_circle(buf: &mut CanvasBuffer, x: i32, y: i32, width: LineWidth, color: Color) {
    for (y_delta, &width) in width.scanlines().iter().enumerate() {
        let y_delta = y_delta as i32;
        let width = i32::from(width);
        let x1 = x - (width - 1) / 2;
        let x2 = x1 + width;
        draw_scanline(buf, x1, x2, y - y_delta, color);
        draw_scanline(buf, x1, x2, y + y_delta, color);
    }
}

fn draw_scanline(buf: &mut CanvasBuffer, x1: i32, x2: i32, y: i32, color: Color) {
    if let Ok(y) = usize::try_from(y) {
        for x in x1..=x2 {
            if let Ok(x) = usize::try_from(x) {
                buf.set(x, y, color);
            }
        }
    }
}
