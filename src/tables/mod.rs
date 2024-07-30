// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

macro_rules! create_table {
    {$($tag:ident),* $(,)*} => {
        paste::paste! {
            $(
                pub mod [<$tag:lower>];
            )*

            #[derive(Debug)]
            pub enum Table<A: core::alloc::Allocator + core::fmt::Debug + 'static> {
                $(
                    [<$tag:camel>]([<$tag:lower>]::ParsedType<A>),
                )*
            }

            pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug + 'static, R: crate::types::CoreRead>(
                allocator: A,
                prev_tables: &[Table<A>],
                tag: [u8; 4],
                reader: &mut R,
            ) -> Result<Table<A>, crate::FontError<R::IoError>> {
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


                    match tag {
                        $(
                            [<$tag:upper>] => Ok(Table::[<$tag:camel>]([<$tag:lower>]::parse_table(allocator, prev_tables, reader)?)),
                        )*
                        _ => Err(crate::FontError::InvalidTag(tag))
                    }

            }
        }
    };
}

create_table! {
    glyf, maxp, loca, head, name
}
