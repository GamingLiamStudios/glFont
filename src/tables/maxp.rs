// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use std::marker::PhantomData;

use super::Table;
use crate::{
    io::CoreVec,
    Error,
};

pub type ParsedType<A> = Type<A>;

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
pub fn parse_table<A: core::alloc::Allocator + Copy, IoError>(
    _allocator: A,
    _prev_tables: &[Table<A>],
    data: CoreVec<u8, A>,
) -> Result<Type<A>, Error<IoError>> {
    // Must be at least 6 bytes (v16d16 + u16)
    let packed_ver =
        u32::from_be_bytes(data[..4].try_into().map_err(|_| Error::UnexpectedEop {
            location: "maxp",
            needed:   4 - data.len(),
        })?);

    match packed_ver {
        0x0000_5000 => {
            // Version 0.5
            tracing::event!(tracing::Level::TRACE, "Version 0.5");
            Ok(Type::Ver05 {
                num_glyphs: u16::from_be_bytes(data[4..6].try_into().map_err(|_| {
                    Error::UnexpectedEop {
                        location: "maxp",
                        needed:   6 - data.len(),
                    }
                })?),
            })
        },
        0x0001_0000 => {
            // Version 1.0
            tracing::event!(tracing::Level::TRACE, "Version 1.0");
            Ok(Type::Ver10 {
                num_glyphs: u16::from_be_bytes(data[4..6].try_into().map_err(|_| {
                    Error::UnexpectedEop {
                        location: "maxp",
                        needed:   6 - data.len(),
                    }
                })?),
            })
        },
        _ => Err(Error::InvalidVersion {
            location: "maxp",
            version:  packed_ver,
        }),
    }
}
