// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use super::Table;
use crate::{
    types::{
        CoreRead,
        CoreVec,
    },
    ParseError,
};

pub type ParsedType<A> = CoreVec<Type, A>;

#[derive(Debug)]
pub struct Type {
    advance:           u16,
    left_side_bearing: i16,
}

#[tracing::instrument(skip_all, level = "trace")]
pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: CoreRead>(
    allocator: A,
    prev_tables: &[Table<A>],
    reader: &mut R,
) -> Result<ParsedType<A>, ParseError<R::IoError>> {
    let Some(Table::Maxp(maxp)) = prev_tables.iter().find(|v| matches!(v, Table::Maxp(_))) else {
        return Err(ParseError::MissingTable {
            missing: "maxp",
            parsing: "hmtx",
        });
    };

    let Some(Table::Hhea(hhea)) = prev_tables.iter().find(|v| matches!(v, Table::Hhea(_))) else {
        return Err(ParseError::MissingTable {
            missing: "hhea",
            parsing: "hmtx",
        });
    };

    let num_glyphs = *match maxp {
        super::maxp::Type::Ver05 { num_glyphs } | super::maxp::Type::Ver10 { num_glyphs, .. } => {
            num_glyphs
        },
        super::maxp::Type::_Phantom(_) => unreachable!(),
    } as usize;

    let mut metrics = CoreVec::with_capacity_in(num_glyphs, allocator);

    for _ in 0..hhea.num_hmetric {
        let advance: u16 = reader.read_int()?;
        let left_side_bearing: i16 = reader.read_int()?;
        metrics.push(Type {
            advance,
            left_side_bearing,
        });
    }

    let advance = metrics
        .last()
        .expect("No horizontal metrics in hmtx")
        .advance;
    for _ in 0..num_glyphs - usize::from(hhea.num_hmetric) {
        let left_side_bearing: i16 = reader.read_int()?;
        metrics.push(Type {
            advance,
            left_side_bearing,
        });
    }

    Ok(metrics)
}
