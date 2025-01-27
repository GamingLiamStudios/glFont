// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use super::Table;
use crate::{
    types::{
        CoreRead,
        CoreVec,
        TrackingReader,
    },
    ParseError,
};

pub type ParsedType<A> = CoreVec<Glyph<A>, A>;

pub struct Flags;
impl Flags {
    pub const ON_CURVE: u8 = 1 << 0;
    pub const REPEAT: u8 = 1 << 3;
    pub const X_SHORT: u8 = 1 << 1;
    // Determines sign if Short, to skip if Long
    pub const X_SIGN_SKIP: u8 = 1 << 4;
    pub const Y_SHORT: u8 = 1 << 2;
    pub const Y_SIGN_SKIP: u8 = 1 << 5;
}

#[derive(Debug, Clone)]
pub struct Glyph<A: core::alloc::Allocator> {
    pub num_contours: i16,

    pub x_bounds: core::range::RangeInclusive<i16>,
    pub y_bounds: core::range::RangeInclusive<i16>,

    // Simple Glyph
    pub end_pts: CoreVec<u16, A>,
    // (x, y, on_curve)
    pub points:  CoreVec<(i16, i16, bool), A>,
    // instructions: io::CoreVec<u8, A>, // TODO: Parse bytecodes
}

macro_rules! read_coords {
    ($allocator:ident $reader:ident $flags:ident $type:ident) => {{
        paste::paste! {
            let mut vec = CoreVec::with_capacity_in($flags.len(), $allocator);
            for flags in &$flags {
                vec.push(
                    match (
                        *flags & Flags::[<$type _SHORT>] != 0,
                        *flags & Flags::[<$type _SIGN_SKIP>] != 0,
                    ) {
                        (true, false) => -i16::from($reader.read_int::<u8>()?),
                        (true, true) => i16::from($reader.read_int::<u8>()?),
                        (false, false) => $reader.read_int::<i16>()?,
                        (false, true) => 0i16,
                    },
                );
            }

            vec.shrink_to_fit();
            vec
        }
    }};
}

pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug + 'static, R: CoreRead>(
    allocator: A,
    prev_tables: &[Table<A>],
    reader: &mut R,
) -> Result<ParsedType<A>, ParseError<R::IoError>> {
    // Requires `maxp` + `loca` tables
    let Some(Table::Loca(loca)) = prev_tables.iter().find(|v| matches!(v, Table::Loca(_))) else {
        return Err(ParseError::MissingTable {
            missing: "loca",
            parsing: "glyf",
        });
    };

    let mut glyphs = CoreVec::with_capacity_in(loca.len(), allocator);
    let mut reader = TrackingReader::new(reader);

    let mut prev_complex = false;
    for idx in 0..loca.len() {
        let (offset, len) = loca.index(idx);
        if len == 0 {
            //println!("Empty contour {idx}");
            glyphs.push(Glyph {
                num_contours: 0,
                x_bounds:     (0..=0).into(),
                y_bounds:     (0..=0).into(),
                end_pts:      CoreVec::new_in(allocator),
                points:       CoreVec::new_in(allocator),
            });
            continue;
        }

        if !prev_complex && offset as usize != reader.total_read() {
            tracing::event!(
                tracing::Level::DEBUG,
                "Index mismatch! expected {} got {}",
                loca.index(idx).0,
                reader.total_read()
            );
        }
        let _ = reader.skip(loca.index(idx).0 as usize - reader.total_read())?;
        if reader.total_read() != loca.index(idx).0 as usize {
            return Err(ParseError::UnexpectedEop {
                location: "glyf",
                needed:   loca.index(idx).0 as usize - reader.total_read(),
            });
        }
        prev_complex = false;

        let num_contours: i16 = reader.read_int()?;

        let x_min: i16 = reader.read_int()?;
        let y_min: i16 = reader.read_int()?;
        let x_max: i16 = reader.read_int()?;
        let y_max: i16 = reader.read_int()?;

        let x_bounds = core::range::RangeInclusive::from(x_min..=x_max);
        let y_bounds = core::range::RangeInclusive::from(y_min..=y_max);

        if num_contours == 0 {
            tracing::event!(tracing::Level::TRACE, "No contours");
        }

        if num_contours < 0 {
            // TODO: Implement Composite Glyphs
            // For now, we're gonna duplicate the 0th glyph (NULL_CHAR)
            glyphs.push(glyphs[0].clone());
            prev_complex = true;
            continue;
        }

        // Read end_pts bytes
        let mut end_pts = CoreVec::with_capacity_in(
            usize::try_from(num_contours).expect("Signed to Unsigned cast failed"),
            allocator,
        );
        for _ in 0..num_contours {
            end_pts.push(reader.read_int()?);
        }

        let num_instructions: u16 = reader.read_int()?;
        for _ in 0..num_instructions {
            let _instruction: u8 = reader.read_int()?;
            // TODO: Parse
        }

        // flags has to be handled manually as we need to duplicate the repeats
        let num_points = usize::from(*end_pts.last().expect("No points in Glyph")) + 1;

        let mut flags_vec = CoreVec::with_capacity_in(num_points, allocator);
        while flags_vec.len() != num_points {
            let flags: u8 = reader.read_int()?;

            let mut repeat = 1;
            if flags & Flags::REPEAT != 0 {
                repeat += u16::from(reader.read_int::<u8>()?);
            }

            for _ in 0..repeat {
                flags_vec.push(flags & !Flags::REPEAT);
            }
        }
        flags_vec.shrink_to_fit();

        // in the quest for lower LOC count
        let x_coords = read_coords!(allocator reader flags_vec X);
        let y_coords = read_coords!(allocator reader flags_vec Y);

        let mut points = CoreVec::with_capacity_in(num_points, allocator);
        points.extend(
            itertools::izip!(flags_vec, x_coords, y_coords)
                .map(|(f, x, y)| (x, y, f & Flags::ON_CURVE != 0)),
        );

        glyphs.push(Glyph {
            num_contours,
            x_bounds,
            y_bounds,
            end_pts,
            points,
        });
    }

    glyphs.shrink_to_fit();
    Ok(glyphs)
}
