// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use super::Table;
use crate::types::{
    CoreRead,
    CoreVec,
    Error,
    ValidType,
};

pub type ParsedType<A> = Type<A>;

#[derive(Debug)]
pub struct Type<A: core::alloc::Allocator> {
    offsets: CoreVec<u32, A>,
}

impl<A: core::alloc::Allocator> Type<A> {
    pub fn index(
        &self,
        idx: usize,
    ) -> (u32, u32) {
        assert!(idx < self.offsets.len() - 1);
        (self.offsets[idx], self.offsets[idx + 1] - self.offsets[idx])
    }

    pub fn len(&self) -> usize {
        self.offsets.len() - 1
    }
}

#[tracing::instrument(skip_all, level = "trace")]
pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: CoreRead>(
    allocator: A,
    prev_tables: &[Table<A>],
    reader: &mut R,
) -> Result<Type<A>, Error<R::IoError>> {
    let Some(Table::Head(head)) = prev_tables.iter().find(|v| matches!(v, Table::Head(_))) else {
        return Err(crate::Error::MissingTable {
            missing: "head",
            parsing: "loca",
        });
    };

    let Some(Table::Maxp(maxp)) = prev_tables.iter().find(|v| matches!(v, Table::Maxp(_))) else {
        return Err(crate::Error::MissingTable {
            missing: "maxp",
            parsing: "loca",
        });
    };

    let num_glyphs = *match maxp {
        super::maxp::Type::Ver05 { num_glyphs } | super::maxp::Type::Ver10 { num_glyphs, .. } => {
            num_glyphs
        },
        super::maxp::Type::_Phantom(_) => unreachable!(),
    } as usize
        + 1;

    let mut offsets = CoreVec::with_capacity_in(num_glyphs, allocator);
    offsets.resize(num_glyphs, 0);

    let mut prev = u32::MIN;
    if head.long_offset {
        // u32, offset
        for offset in &mut offsets {
            *offset = reader.read_int()?;

            // loca[n + 1] >= loca[n]
            if *offset < prev {
                return Err(Error::Parsing {
                    variable: "loca",
                    expected: ValidType::U32(prev + 1),
                    parsed:   ValidType::U32(*offset),
                });
            }
            prev = *offset;
        }
    } else {
        // u16, offset / 2
        for offset in &mut offsets {
            let half: u16 = reader.read_int()?;
            *offset = u32::from(half) * 2;

            // loca[n + 1] >= loca[n]
            if *offset < prev {
                return Err(Error::Parsing {
                    variable: "loca",
                    expected: ValidType::U32(prev + 1),
                    parsed:   ValidType::U32(*offset),
                });
            }
            prev = *offset;
        }
    }

    tracing::event!(tracing::Level::DEBUG, "NumGlyphs {}", offsets.len() - 1);

    Ok(Type { offsets })
}
