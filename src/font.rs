// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
use crate::{
    tables::{
        name::RecordType,
        Table,
    },
    types::CoreVec,
};

// A: core::alloc::Allocator + core::fmt::Debug + 'static
pub type Font<A = alloc::alloc::Global> = CoreVec<Table<A>, A>;

pub trait Trait {
    fn name_record(
        &self,
        record_type: RecordType,
    ) -> Option<&str>;
}

impl<A: core::alloc::Allocator + core::fmt::Debug + 'static> Trait for Font<A> {
    fn name_record(
        &self,
        record_type: RecordType,
    ) -> Option<&str> {
        let Some(Table::Name(name_table)) = self.iter().find(|t| matches!(t, Table::Name(_)))
        else {
            return None;
        };

        name_table
            .records
            .iter()
            .find(|r| r.name == record_type)
            .map(|r| r.string.as_ref())
    }
}
