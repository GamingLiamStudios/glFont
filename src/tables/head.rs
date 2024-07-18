// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use std::marker::PhantomData;

use super::Table;
use crate::{
    io::CoreVec,
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

pub struct Type<A: core::alloc::Allocator> {
    pub units_per_em:     u16,
    pub smallest_px_size: u16,
    pub style:            u16,
    pub long_offset:      bool,

    _phantom: PhantomData<A>,
}

#[tracing::instrument(skip_all, level = "trace")]
pub fn parse_table<A: core::alloc::Allocator + Copy, IoError>(
    _allocator: A,
    _prev_tables: &[Table<A>],
    data: CoreVec<u8, A>,
) -> Result<Type<A>, Error<IoError>> {
    let major_version =
        u16::from_be_bytes(data[..2].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::majorVersion",
            needed:   2 - data.len(),
        })?);
    let minor_version =
        u16::from_be_bytes(data[2..4].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::minorVersion",
            needed:   4 - data.len(),
        })?);

    if major_version != 1 && minor_version != 0 {
        return Err(Error::InvalidVersion {
            location: "head",
            version:  (u32::from(major_version) << u16::BITS) | u32::from(minor_version),
        });
    }

    let font_revision =
        fixed::types::I16F16::from_be_bytes(data[4..8].try_into().map_err(|_| {
            Error::UnexpectedEop {
                location: "head::fontRevision",
                needed:   8 - data.len(),
            }
        })?);
    tracing::event!(tracing::Level::DEBUG, "Font Revision: {font_revision}");

    // 8..12 - Skip checksumAdjustment, already verified

    let magic = u32::from_be_bytes(data[12..16].try_into().map_err(|_| Error::UnexpectedEop {
        location: "head::magic",
        needed:   12 - data.len(),
    })?);
    if magic != 0x5f0f_3cf5_u32 {
        return Err(Error::Parsing {
            variable: "head::magic",
            expected: crate::ValidType::U32(0x5f0f_3cf5_u32),
            parsed:   crate::ValidType::U32(magic),
        });
    }

    // 16..18 Skip flags

    let units_per_em =
        u16::from_be_bytes(data[18..20].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::unitsPerEm",
            needed:   20 - data.len(),
        })?);
    if !(16..=16384).contains(&units_per_em) {
        return Err(Error::Parsing {
            variable: "head::unitsPerEm",
            expected: crate::ValidType::U16(if units_per_em < 16 { 16 } else { 16384 }),
            parsed:   crate::ValidType::U16(units_per_em),
        });
    }

    let created_time =
        i64::from_be_bytes(data[20..28].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::created",
            needed:   28 - data.len(),
        })?);
    let modified_time =
        i64::from_be_bytes(data[28..36].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::modified",
            needed:   36 - data.len(),
        })?);
    tracing::event!(
        tracing::Level::DEBUG,
        "Created: {}",
        crate::ValidType::LDT(created_time)
    );
    tracing::event!(
        tracing::Level::DEBUG,
        "Modified: {}",
        crate::ValidType::LDT(modified_time)
    );

    // 36..38 Skip xMin
    // 38..40 Skip yMin
    // 40..42 Skip xMax
    // 42..44 Skip yMax

    let style = u16::from_be_bytes(data[44..46].try_into().map_err(|_| Error::UnexpectedEop {
        location: "head::macStyle",
        needed:   46 - data.len(),
    })?);

    let smallest_px_size =
        u16::from_be_bytes(data[46..48].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::lowestRecPPEM",
            needed:   48 - data.len(),
        })?);

    // Deprecated
    // 48..50 Skip fontDirectionHint

    let long_offset =
        u16::from_be_bytes(data[50..52].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::indexToLocFormat",
            needed:   52 - data.len(),
        })?);
    if long_offset > 1 {
        return Err(Error::Parsing {
            variable: "head::indexToLocFormat",
            expected: crate::ValidType::U16(1),
            parsed:   crate::ValidType::U16(long_offset),
        });
    }

    let glyf_format =
        u16::from_be_bytes(data[52..54].try_into().map_err(|_| Error::UnexpectedEop {
            location: "head::glyphDataFormat",
            needed:   52 - data.len(),
        })?);
    if glyf_format != 0 {
        return Err(Error::Parsing {
            variable: "head::glyphDataFormat",
            expected: crate::ValidType::U16(0),
            parsed:   crate::ValidType::U16(glyf_format),
        });
    }

    Ok(Type {
        units_per_em,
        style,
        smallest_px_size,
        long_offset: long_offset == 1,
        _phantom: PhantomData {},
    })
}
