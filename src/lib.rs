// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
//#![feature(const_mut_refs)]
//#![feature(const_fn_floating_point_arithmetic)]
//#![no_std]
#![feature(allocator_api)] // no_std
#![feature(generic_const_exprs)] // CoreRead::read_int
#![feature(new_range_api)] // glyf
#![feature(new_uninit)] // name
#![allow(clippy::missing_errors_doc)]
#![allow(incomplete_features)]

extern crate alloc;

mod font;
mod tables;
mod types;

pub use font::Trait as FontTrait;
pub use tables::name::RecordType as NameRecord;
use types::{
    ChecksumReader,
    CoreRead,
    CoreVec,
    Error,
};

fn verify_header<R: CoreRead>(input: &mut R) -> Result<u16, Error<R::IoError>> {
    let mut version = [0; 4];
    input.read(&mut version)?;
    if version != [0x00, 0x01, 0x00, 0x00] {
        return Err(Error::InvalidSfntVersion(version));
    }

    let num_tables: u16 = input.read_int()?;
    tracing::trace!("NumTables: {num_tables}");

    let search_range: u16 = input.read_int()?;
    if search_range != 2_u16.pow(num_tables.ilog2()) * 16 {
        return Err(Error::Parsing {
            variable: "SearchRange",
            expected: types::ValidType::U16(2_u16.pow(num_tables.ilog2()) * 16),
            parsed:   types::ValidType::U16(search_range),
        });
    }

    let entry_selector: u16 = input.read_int()?;
    if entry_selector != u16::try_from(num_tables.ilog2()).expect("ilog2 downcast failed") {
        return Err(Error::Parsing {
            variable: "EntrySelector",
            expected: types::ValidType::U32(num_tables.ilog2()),
            parsed:   types::ValidType::U16(entry_selector),
        });
    }

    let range_shift: u16 = input.read_int()?;
    if range_shift != num_tables * 16 - search_range {
        return Err(Error::Parsing {
            variable: "RangeShift",
            expected: types::ValidType::U16(num_tables * 16 - search_range),
            parsed:   types::ValidType::U16(range_shift),
        });
    }

    Ok(num_tables)
}

/// # Panics
/// - If Slice of size `N` is unable to cast to array of type `[u8; N]`
/// - If Downcast fails
#[tracing::instrument(level = "trace", skip_all)]
pub fn open_font<A: core::alloc::Allocator + Copy + core::fmt::Debug + 'static, R: CoreRead>(
    allocator: A,
    input: &mut R,
) -> Result<font::Font<A>, Error<R::IoError>> {
    let mut reader = ChecksumReader::new(input);

    let num_tables = verify_header(&mut reader)?;
    let mut tables = CoreVec::with_capacity_in(num_tables as usize, allocator);

    for _ in 0..num_tables {
        let mut tag = [0u8; 4];
        let read = reader.read(&mut tag)?;
        if read != tag.len() {
            return Err(Error::UnexpectedEop {
                location: "TableRecord",
                needed:   tag.len() - read,
            });
        }

        let checksum: u32 = reader.read_int()?;
        let offset: u32 = reader.read_int()?;
        let length: u32 = reader.read_int()?;

        tables.push((tag, checksum, offset as usize, length as usize));
    }

    tracing::event!(
        name: "Header",
        tracing::Level::TRACE,
        "Bytes read: {}", reader.total_read()
    );
    let mut parsed_tables = Vec::new_in(allocator);

    let mut checksum_adj = 0;

    tables.sort_by(|(_, _, a, _), (_, _, b, _)| a.cmp(b));
    for (tag, checksum, offset, length) in tables {
        if offset != reader.total_read() {
            tracing::event!(
                tracing::Level::WARN,
                "Read Mismatch! expected {offset}, got {}",
                reader.total_read()
            );
            // Fix read bytes
            reader.skip(offset - reader.total_read())?;
        }

        let mut tag_reader = ChecksumReader::new(&mut reader);

        tracing::event!(
            tracing::Level::TRACE,
            "Read {}: {length} at {offset}",
            types::ValidType::Tag(tag)
        );

        let parsed = tables::parse_table(allocator, &parsed_tables, tag, &mut tag_reader);

        tag_reader.skip(length - tag_reader.total_read())?;
        let mut checksum_act = tag_reader.finish()?;

        if tag == *b"head" {
            let tables::Table::Head(head) = parsed? else {
                panic!("head not parsed as head");
            };

            // this works cuz it's on a 4-byte boundary
            (checksum_act, _) = checksum_act.overflowing_sub(head.checksum_adjustment);
            checksum_adj = head.checksum_adjustment;

            parsed_tables.push(tables::Table::Head(head));
        } else if parsed.is_ok() {
            parsed_tables.push(parsed?);
        } else {
            let error = parsed.expect_err("is_not_ok");
            if !matches!(error, Error::InvalidTag(_)) {
                return Err(error);
            };
        }

        if checksum_act != checksum {
            tracing::event!(
                tracing::Level::TRACE,
                "Checksum invalid! {checksum} != {checksum_act}"
            );
            /*
            return Err(Error::Parsing {
                variable: "TableChecksum",
                expected: ValidType::U32(checksum),
                parsed:   ValidType::U32(checksum_act),
            });
            */
        }
    }

    let mut checksum = reader.finish()?;

    if let Some(tables::Table::Head(head)) = parsed_tables
        .iter()
        .find(|t| matches!(t, tables::Table::Head(_)))
    {
        (checksum, _) = checksum.overflowing_sub(head.checksum_adjustment);
    }

    // ChecksumAdjustment may be set to 0 for version 'OTTO'
    (checksum, _) = 0xb1b0_afba_u32.overflowing_sub(checksum);
    if checksum != checksum_adj {
        return Err(Error::Parsing {
            variable: "ChecksumAdjustment",
            expected: types::ValidType::U32(checksum_adj),
            parsed:   types::ValidType::U32(checksum),
        });
    }

    Ok(parsed_tables)
}
