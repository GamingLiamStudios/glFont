// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
use crate::io::CoreVec;

macro_rules! create_table {
    {$($tag:ident),* $(,)*} => {
        paste::paste! {
            $(
                mod [<$tag:lower>];
            )*

            pub enum Table<A: core::alloc::Allocator> {
                _Dummy(CoreVec<u8, A>),
                $(
                    [<$tag:camel>]([<$tag:lower>]::ParsedType<A>),
                )*
            }

            pub fn parse_table<A: core::alloc::Allocator + Copy, IoError>(
                allocator: A,
                prev_tables: &[Table<A>],
                tag: [u8; 4],
                data: CoreVec<u8, A>,
            ) -> Result<Table<A>, crate::Error<IoError>> {
                $(
                    const [<$tag:upper>]: [u8; stringify!([<$tag:lower>]).len()] = {
                        const BYTES: &[u8] = stringify!([<$tag:lower>]).as_bytes();
                        let mut result: [u8; BYTES.len()] = [0u8; BYTES.len()];
                        let mut idx = 0;
                        while idx < BYTES.len() {
                            result[idx] = BYTES[idx];
                            idx += 1;
                        }
                        result
                    };
                )*

                Ok(
                    match tag {
                        $(
                            [<$tag:upper>] => Table::[<$tag:camel>]([<$tag:lower>]::parse_table(allocator, prev_tables, data)?),
                        )*
                        _ => Table::_Dummy(data),
                    }
                )
            }
        }
    };
}

create_table! {
    glyf, maxp, loca, head
}
