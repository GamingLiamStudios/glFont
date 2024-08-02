// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use core::mem;

use super::{
    Display,
    DrawMode,
};

#[allow(clippy::cast_sign_loss)]
pub fn draw_pixel<T: num_traits::PrimInt>(
    display: &mut Display<'_, T>,
    draw_mode: DrawMode,
    x: i32,
    y: i32,
    value: f32,
) {
    let height = display.buffer.len() / display.width;
    if x < 0 || y < 0 || (x as usize) >= display.width || (y as usize) >= height {
        return;
    }
    //println!("{x} {y} {value}");
    let pixel_val: T =
        T::from(T::max_value().to_f32().expect("overflow on T to f32") * value.abs())
            .unwrap_or_else(|| T::max_value());

    let prev_pixel = display.buffer[y as usize * display.width + x as usize].0;
    display.buffer[y as usize * display.width + x as usize] = rgb::Gray::new(match draw_mode {
        DrawMode::Overwrite => pixel_val,
        DrawMode::Multiply => prev_pixel * pixel_val,
        DrawMode::Add => prev_pixel.saturating_add(pixel_val),
    });
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn draw_line<T: num_traits::PrimInt>(
    display: &mut Display<'_, T>,
    start: (f32, f32),
    end: (f32, f32),
    width: f32,
) {
    let half_width = width / 4.0;

    let (mut x1, mut y1) = start;
    let (mut x2, mut y2) = end;

    let is_steep = (y2 - y1).abs() > (x2 - x1).abs();
    if is_steep {
        mem::swap(&mut x1, &mut y1);
        mem::swap(&mut x2, &mut y2);
    }
    if x1 > x2 {
        mem::swap(&mut x1, &mut x2);
        mem::swap(&mut y1, &mut y2);
    }

    // draw rectangle outline
    let dx = x2 - x1;
    let dy = y2 - y1;
    let slope = if dx == 0f32 { 1f32 } else { dy / dx };

    // perp line = end lines
    // perp: m1 x m2 = -1
    // -1/m1 = m2
    // -1 / slope = perp_slope
    let perp_slope = -1f32 / slope;

    let to_coords = |x: f32, c: f32| {
        if is_steep {
            (perp_slope.mul_add(x, c), x)
        } else {
            (x, perp_slope.mul_add(x, c))
        }
    };

    let mut y = slope.mul_add(x1.round() - x1, y1) + slope;

    let scale_factor = width * 4.0;
    for x in ((x1.round() * scale_factor) as i32)..=((x2.round() * scale_factor) as i32) {
        let x = x as f32 / scale_factor;
        let c = perp_slope.mul_add(-x, y);
        draw_line_antialiased(
            display,
            DrawMode::Add,
            to_coords(x - half_width, c),
            to_coords(x + half_width, c),
        );
        y += slope / scale_factor;
    }
}

pub fn draw_line_aliased<T: num_traits::PrimInt>(
    display: &mut Display<'_, T>,
    draw_mode: DrawMode,
    start: (i32, i32),
    end: (i32, i32),
) {
    let (mut x1, mut y1) = start;
    let (x2, y2) = end;

    let dx = (x2 - x1).abs();
    let dy = -(y2 - y1).abs();

    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };

    let mut error = dx + dy;
    loop {
        draw_pixel(display, draw_mode, x1, y1, 1f32);
        if x1 == x2 && y1 == y2 {
            break;
        }

        let e2 = 2 * error;
        if e2 >= dy {
            if x1 == x2 {
                break;
            }
            error += dy;
            x1 += sx;
        }
        if e2 <= dx {
            if y1 == y2 {
                break;
            }
            error += dx;
            y1 += sy;
        }
    }
}

/// [f32] coords should be fine, but if an image is larger than `u16::MAX`,
/// precision issues might occur
#[allow(clippy::cast_possible_truncation)]
pub fn draw_line_antialiased<T: num_traits::PrimInt>(
    display: &mut Display<'_, T>,
    draw_mode: DrawMode,
    start: (f32, f32),
    end: (f32, f32),
) {
    let (mut x1, mut y1) = start;
    let (mut x2, mut y2) = end;

    let is_steep = (y2 - y1).abs() > (x2 - x1).abs();
    if is_steep {
        mem::swap(&mut x1, &mut y1);
        mem::swap(&mut x2, &mut y2);
    }
    if x1 > x2 {
        mem::swap(&mut x1, &mut x2);
        mem::swap(&mut y1, &mut y2);
    }

    let dx = x2 - x1;
    let dy = y2 - y1;
    let gradient = if dx == 0f32 { 1f32 } else { dy / dx };

    // first endpoint
    let (x_px1, y_px1, mut y_int, ev1) = {
        let x_end = x1.round();
        let y_end = gradient.mul_add(x_end - x1, y1);

        (
            x_end as i32,
            y_end.trunc() as i32,
            y_end + gradient,
            y_end.fract(),
        )
    };

    // second_endpoint
    let (x_px2, y_px2, ev2) = {
        let x_end = x2.round();
        let y_end = gradient.mul_add(x_end - x2, y2);

        (x_end as i32, y_end.trunc() as i32, y_end.fract())
    };

    let mut draw_steep_pixel = |x: i32, y: i32, v: f32| {
        if is_steep {
            draw_pixel(display, draw_mode, y, x, v);
        } else {
            draw_pixel(display, draw_mode, x, y, v);
        }
    };

    // Render end points
    let x_gap = 1f32 - (x1 + 0.5f32).fract();
    draw_steep_pixel(x_px1, y_px1, (1f32 - ev1) * x_gap);
    draw_steep_pixel(x_px1, y_px1 + 1, ev1 * x_gap);

    let x_gap = 1f32 - (x2 + 0.5f32).fract();
    draw_steep_pixel(x_px2, y_px2, (1f32 - ev2) * x_gap);
    draw_steep_pixel(x_px2, y_px2 + 1, ev2 * x_gap);

    for x in (x_px1 + 1)..x_px2 {
        draw_steep_pixel(x, y_int.trunc() as i32, 1f32 - y_int.fract());
        draw_steep_pixel(x, y_int.trunc() as i32 + 1, y_int.fract());
        y_int += gradient;
    }
}
