// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
use std::marker::PhantomData;

use super::Table;
use crate::{
    io,
    Error,
};

pub type ParsedType<A> = Type<A>;

#[derive(Debug)]
pub struct Type<A: core::alloc::Allocator> {
    _phantom: PhantomData<A>,
}

pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: io::CoreRead>(
    allocator: A,
    prev_tables: &[Table<A>],
    reader: &R,
) -> Result<Type<A>, Error<R::IoError>> {
    let Some(Table::Head(head)) = prev_tables.iter().find(|v| matches!(v, Table::Head(_))) else {
        return Err(crate::Error::MissingTable {
            missing: "head",
            parsing: "glyf",
        });
    };

    Ok(Type {
        _phantom: PhantomData {},
    })
}
