// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
use super::Table;
use crate::io::CoreVec;

pub type ParsedType<A> = CoreVec<Type<A>, A>;

pub struct Type<A: core::alloc::Allocator> {
    data: CoreVec<u8, A>,
}

pub fn parse_table<A: core::alloc::Allocator + Copy, IoError>(
    allocator: A,
    prev_tables: &[Table<A>],
    data: CoreVec<u8, A>,
) -> Result<CoreVec<Type<A>, A>, crate::Error<IoError>> {
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

    Ok(CoreVec::new_in(allocator))
}
