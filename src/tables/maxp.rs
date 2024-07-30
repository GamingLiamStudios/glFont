// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use core::marker::PhantomData;

use super::Table;
use crate::{
    types::CoreRead,
    FontError,
};

pub type ParsedType<A> = Type<A>;

#[derive(Debug)]
pub enum Type<A: core::alloc::Allocator> {
    Ver05 {
        num_glyphs: u16,
    },
    Ver10 {
        num_glyphs: u16,
    },

    #[doc(hidden)]
    _Phantom(PhantomData<A>),
}

#[tracing::instrument(skip_all, level = "trace")]
pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: CoreRead>(
    _allocator: A,
    _prev_tables: &[Table<A>],
    reader: &mut R,
) -> Result<Type<A>, FontError<R::IoError>> {
    // Must be at least 6 bytes (v16d16 + u16)
    let packed_ver: u32 = reader.read_int()?;

    match packed_ver {
        0x0000_5000 => {
            // Version 0.5
            tracing::event!(tracing::Level::TRACE, "Version 0.5");
            Ok(Type::Ver05 {
                num_glyphs: reader.read_int()?,
            })
        },
        0x0001_0000 => {
            // Version 1.0
            tracing::event!(tracing::Level::TRACE, "Version 1.0");
            Ok(Type::Ver10 {
                num_glyphs: reader.read_int()?,
            })
        },
        _ => Err(FontError::InvalidVersion {
            location: "maxp",
            version:  packed_ver,
        }),
    }
}
