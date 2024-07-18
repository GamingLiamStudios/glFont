// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
use std::marker::PhantomData;

use super::Table;
use crate::io;

pub type ParsedType<A> = io::CoreVec<Type<A>, A>;

#[derive(Debug)]
pub struct Type<A: core::alloc::Allocator> {
    _phantom: PhantomData<A>,
}

pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: io::CoreRead>(
    allocator: A,
    prev_tables: &[Table<A>],
    reader: &R,
) -> Result<io::CoreVec<Type<A>, A>, crate::Error<R::IoError>> {
    // Requires `maxp` + `loca` tables
    let Some(Table::Maxp(maxp)) = prev_tables.iter().find(|v| matches!(v, Table::Maxp(_))) else {
        return Err(crate::Error::MissingTable {
            missing: "maxp",
            parsing: "glyf",
        });
    };

    let Some(Table::Loca(loca)) = prev_tables.iter().find(|v| matches!(v, Table::Loca(_))) else {
        return Err(crate::Error::MissingTable {
            missing: "loca",
            parsing: "glyf",
        });
    };

    let num_glyphs = *match maxp {
        super::maxp::Type::Ver05 { num_glyphs } | super::maxp::Type::Ver10 { num_glyphs, .. } => {
            num_glyphs
        },
        super::maxp::Type::_Phantom(_) => unreachable!(),
    };

    let mut glyphs = io::CoreVec::with_capacity_in(num_glyphs as usize, allocator);
    //glyphs.push(Type {  });

    Ok(glyphs)
}
