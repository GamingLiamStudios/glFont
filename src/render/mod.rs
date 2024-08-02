// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use itertools::Itertools;
use num_traits::PrimInt;

mod shapes;

use crate::{
    types::{
        CoreVec,
        SlotmapKey,
    },
    FontCollection,
    FontTrait,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {}

#[derive(Debug)]
pub enum SubPixelAlignment {
    Rgb,
    Bgr,
    None,
}

#[derive(Debug, Copy, Clone)]
pub enum DrawMode {
    Overwrite,
    Multiply,
    Add,
}

pub type FormattedText<'a, A> = CoreVec<FormattedSlice<'a>, A>;

#[derive(Debug)]
pub struct FormattedSlice<'a> {
    pub id:   SlotmapKey,
    pub size: u16,
    pub text: &'a str,
}

struct Display<'a, T: PrimInt> {
    pub buffer: &'a mut [rgb::Gray<T>],
    pub width:  usize,

    pub dpi:      u16,
    pub subpixel: SubPixelAlignment,
}

/// # Errors
/// # Panics
#[allow(clippy::cast_possible_truncation)]
pub fn to_buf<A: core::alloc::Allocator + core::fmt::Debug, T: PrimInt>(
    fonts: &FontCollection<A>,
    input: &[FormattedSlice<'_>],
    buffer: &mut [rgb::Gray<T>],
    width: usize,
    dpi: u16,
    subpixel: SubPixelAlignment,
) -> Result<(), Error> {
    let _height = buffer.len() / width;

    for v in buffer.iter_mut() {
        *v = rgb::Gray::new(T::min_value());
    }

    let mut display = Display {
        buffer,
        width,
        dpi,
        subpixel,
    };

    //shapes::draw_line_aliased(&mut display, DrawMode::Overwrite, (0, 0), (99,
    // 99));
    //shapes::draw_line(&mut display, (0.0, 0.0), (200.0, 49.0), 5.0);

    let font = fonts.get(input[0].id);
    let glyph = font.glyph(1).expect("e");

    let units_per_em = f32::from(font.units_per_em());
    let ppem = f32::from(dpi) / 6.0;

    for slice in input {
        let scale = (f32::from(slice.size) / 12.0 * ppem) / units_per_em;
        //println!("{}pt {units_per_em} {ppem}", slice.size);

        let mut prev_x = 0;
        let mut prev_y = 0;

        let mut prev_end = 0;
        //println!("{:?}", glyph.end_pts);
        for end in &glyph.end_pts {
            let start_x = prev_x;
            let start_y = prev_y;

            for (start, end) in (prev_end..=*end).circular_tuple_windows() {
                let (x1, y1, _on_curve1) = glyph.points[start as usize];
                let (mut x2, mut y2, _on_curve2) = glyph.points[end as usize];

                //println!("{start}: ({x1}, {y1})");
                prev_x += x1;
                prev_y += y1;

                if end == prev_end {
                    x2 += start_x;
                    y2 += start_y;
                } else {
                    x2 += prev_x;
                    y2 += prev_y;
                }

                //println!("{start} {end}: ({prev_x}, {prev_y}) -> ({x2}, {y2})");

                let start = (
                    (f32::from(prev_x) * scale) as i32,
                    f32::from(prev_y).mul_add(-scale, 40.0) as i32,
                );
                let end = (
                    (f32::from(x2) * scale) as i32,
                    f32::from(y2).mul_add(-scale, 40.0) as i32,
                );

                //println!("{start:?} {end:?}");

                shapes::draw_line_aliased(&mut display, DrawMode::Overwrite, start, end);
            }
            prev_end = *end + 1;
        }
    }

    Ok(())
}
