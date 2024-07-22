// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use std::marker::PhantomData;

use super::Table;
use crate::{
    types,
    Error,
};

pub type ParsedType<A> = Type<A>;

struct Style;
impl Style {
    const BOLD: u16 = 1 << 0;
    const CONDENSED: u16 = 1 << 5;
    const EXTENDED: u16 = 1 << 6;
    const ITALIC: u16 = 1 << 1;
    const OUTLINE: u16 = 1 << 3;
    const SHADOW: u16 = 1 << 4;
    const UNDERLINE: u16 = 1 << 2;
}

#[derive(Debug)]
pub struct Type<A: core::alloc::Allocator> {
    pub units_per_em:        u16,
    pub smallest_px_size:    u16,
    pub style:               u16,
    pub long_offset:         bool,
    pub checksum_adjustment: u32,

    _phantom: PhantomData<A>,
}

#[tracing::instrument(skip_all, level = "trace")]
pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: crate::io::CoreRead>(
    _allocator: A,
    _prev_tables: &[Table<A>],
    reader: &mut R,
) -> Result<Type<A>, Error<R::IoError>> {
    let major_version: u16 = reader.read_int()?;
    let minor_version: u16 = reader.read_int()?;

    if major_version != 1 && minor_version != 0 {
        return Err(Error::InvalidVersion {
            location: "head",
            version:  (u32::from(major_version) << u16::BITS) | u32::from(minor_version),
        });
    }

    let mut font_revision = [0u8; 4];
    let read = reader.read(&mut font_revision)?;
    if read != font_revision.len() {
        return Err(Error::UnexpectedEop {
            location: "head::fontRevision",
            needed:   font_revision.len() - read,
        });
    }
    let font_revision = fixed::types::I16F16::from_be_bytes(font_revision);
    tracing::event!(tracing::Level::DEBUG, "Font Revision: {font_revision}");

    let checksum_adjustment: u32 = reader.read_int()?;

    let magic: u32 = reader.read_int()?;
    if magic != 0x5f0f_3cf5_u32 {
        return Err(Error::Parsing {
            variable: "head::magic",
            expected: types::ValidType::U32(0x5f0f_3cf5_u32),
            parsed:   types::ValidType::U32(magic),
        });
    }

    // 16..18 Skip flags
    reader.skip(2)?;

    let units_per_em: u16 = reader.read_int()?;
    if !(16..=16384).contains(&units_per_em) {
        return Err(Error::Parsing {
            variable: "head::unitsPerEm",
            expected: types::ValidType::U16(if units_per_em < 16 { 16 } else { 16384 }),
            parsed:   types::ValidType::U16(units_per_em),
        });
    }

    let created_time: i64 = reader.read_int()?;
    let modified_time: i64 = reader.read_int()?;
    tracing::event!(
        tracing::Level::DEBUG,
        "Created: {}",
        types::ValidType::Ldt(created_time)
    );
    tracing::event!(
        tracing::Level::DEBUG,
        "Modified: {}",
        types::ValidType::Ldt(modified_time)
    );

    let _x_min: i16 = reader.read_int()?;
    let _y_min: i16 = reader.read_int()?;
    let _x_max: i16 = reader.read_int()?;
    let _y_max: i16 = reader.read_int()?;

    let style: u16 = reader.read_int()?;
    let smallest_px_size: u16 = reader.read_int()?;

    // Deprecated
    let _font_dir_hint: i16 = reader.read_int()?;

    let long_offset: u16 = reader.read_int()?;
    if long_offset > 1 {
        return Err(Error::Parsing {
            variable: "head::indexToLocFormat",
            expected: types::ValidType::U16(1),
            parsed:   types::ValidType::U16(long_offset),
        });
    }

    let glyf_format: u16 = reader.read_int()?;
    if glyf_format != 0 {
        return Err(Error::Parsing {
            variable: "head::glyphDataFormat",
            expected: types::ValidType::U16(0),
            parsed:   types::ValidType::U16(glyf_format),
        });
    }

    Ok(Type {
        units_per_em,
        style,
        smallest_px_size,
        checksum_adjustment,
        long_offset: long_offset == 1,
        _phantom: PhantomData {},
    })
}
