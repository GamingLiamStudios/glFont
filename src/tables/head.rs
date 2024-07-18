// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
use super::Table;
use crate::{
    io::CoreVec,
    Error,
};

pub type ParsedType<A> = Type<A>;

pub struct Type<A: core::alloc::Allocator> {
    data: CoreVec<u8, A>,
}

pub const fn parse_table<A: core::alloc::Allocator + Copy, IoError>(
    allocator: A,
    prev_tables: &[Table<A>],
    data: CoreVec<u8, A>,
) -> Result<Type<A>, Error<IoError>> {
    Ok(Type { data })
}
