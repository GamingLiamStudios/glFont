// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use core::marker::PhantomData;

use num_traits::ToPrimitive;

use super::Table;
use crate::{
    types::CoreRead,
    FontError,
};

pub type ParsedType<A> = Type<A>;

#[derive(Debug)]
pub enum CaretSlope {
    Vertical,
    Horizontal,
    Specific { rise: i16, run: i16 },
}

#[derive(Debug)]
pub struct Type<A: core::alloc::Allocator> {
    pub max_advance:  u16,
    pub carat_slope:  CaretSlope,
    pub carat_offset: i16,
    pub num_hmetric:  u16,

    _phantom: PhantomData<A>,
}

#[tracing::instrument(skip_all, level = "trace")]
pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: CoreRead>(
    _allocator: A,
    _prev_tables: &[Table<A>],
    reader: &mut R,
) -> Result<Type<A>, FontError<R::IoError>> {
    let major_version: u16 = reader.read_int()?;
    let minor_version: u16 = reader.read_int()?;

    if major_version != 1 || minor_version != 0 {
        return Err(FontError::InvalidVersion {
            location: "hhea",
            version:  (u32::from(major_version) << u16::BITS) | u32::from(minor_version),
        });
    }

    let _ascender: i16 = reader.read_int()?;
    let _descender: i16 = reader.read_int()?;
    let _linegap: i16 = reader.read_int()?;

    let max_advance: u16 = reader.read_int()?;

    let _min_left_side_bearing: i16 = reader.read_int()?;
    let _min_right_side_bearing: i16 = reader.read_int()?;
    let _x_max_extent: i16 = reader.read_int()?;

    let slope_rise: i16 = reader.read_int()?;
    let slope_run: i16 = reader.read_int()?;
    let carat_offset: i16 = reader.read_int()?;

    let carat_slope = match (slope_rise, slope_run) {
        (1, 0) => CaretSlope::Vertical,
        (0, 1) => CaretSlope::Horizontal,
        (rise, run) => CaretSlope::Specific { rise, run },
    };

    // unused
    for _ in 0..4 {
        let _: i16 = reader.read_int()?;
    }

    let data_format: i16 = reader.read_int()?;
    if data_format != 0 {
        return Err(FontError::InvalidVersion {
            location: "hhea",
            version:  u32::try_from(data_format).expect("i16 -> u32 cast failure"),
        });
    }

    let num_hmetric: u16 = reader.read_int()?;

    Ok(Type {
        max_advance,
        carat_slope,
        carat_offset,
        num_hmetric,
        _phantom: PhantomData {},
    })
}
