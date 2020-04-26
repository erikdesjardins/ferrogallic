// http://www.adammil.net/blog/v126_A_More_Efficient_Flood_Fill.html

use ferrogallic_shared::domain::{CanvasBuffer, Color};

pub fn fill(buf: &mut CanvasBuffer, x: usize, y: usize, to: Color) {
    if x < buf.x_len() && y < buf.y_len() {
        let from = buf.get(x, y);
        run_flood_fill(buf, x, y, from, to);
    }
}

fn run_flood_fill(buf: &mut CanvasBuffer, mut x: usize, mut y: usize, from: Color, to: Color) {
    loop {
        let ox = x;
        let oy = y;
        while y != 0 && buf.get(x, y - 1) == from {
            y -= 1;
        }
        while x != 0 && buf.get(x - 1, y) == from {
            x -= 1;
        }
        if x == ox && y == oy {
            break;
        }
    }
    run_flood_fill_core(buf, x, y, from, to);
}

fn run_flood_fill_core(buf: &mut CanvasBuffer, mut x: usize, mut y: usize, from: Color, to: Color) {
    let mut last_row_len = 0;
    loop {
        let mut row_len = 0;
        let mut sx = x;
        if last_row_len != 0 && buf.get(x, y) == to {
            loop {
                last_row_len -= 1;
                if last_row_len == 0 {
                    return;
                }
                x += 1;
                if buf.get(x, y) != to {
                    break;
                }
            }
            sx = x;
        } else {
            loop {
                if x == 0 || buf.get(x - 1, y) != from {
                    break;
                }
                x -= 1;
                buf.set(x, y, to);
                if y != 0 && buf.get(x, y - 1) == from {
                    run_flood_fill(buf, x, y - 1, from, to);
                }
                row_len += 1;
                last_row_len += 1;
            }
        }

        loop {
            if sx >= buf.x_len() || buf.get(sx, y) != from {
                break;
            }
            buf.set(sx, y, to);
            row_len += 1;
            sx += 1;
        }

        if row_len < last_row_len {
            let end = x + last_row_len;
            loop {
                sx += 1;
                if sx >= end {
                    break;
                }
                if buf.get(sx, y) == from {
                    run_flood_fill_core(buf, sx, y, from, to);
                }
            }
        } else if row_len > last_row_len && y != 0 {
            let mut ux = x + last_row_len;
            loop {
                ux += 1;
                if ux >= sx {
                    break;
                }
                if buf.get(ux, y - 1) == from {
                    run_flood_fill(buf, ux, y - 1, from, to);
                }
            }
        }
        last_row_len = row_len;
        y += 1;
        if last_row_len == 0 || y >= buf.y_len() {
            break;
        }
    }
}
