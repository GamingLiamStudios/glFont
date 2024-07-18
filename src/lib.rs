// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
//#![feature(const_mut_refs)]
#![feature(allocator_api)]
#![feature(array_chunks)]
#![feature(const_fn_floating_point_arithmetic)]
#![allow(clippy::missing_errors_doc)]

use std::fmt::Display;

use tables::Table;

mod io;
mod tables;

#[derive(Debug)]
pub enum ValidType {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U24(u32),
    U32(u32),
    I32(i32),

    // 32bit Fixed Point - 16bit fractional
    F16d16(fixed::types::I16F16),

    // 16bit Fixed Point - 14bit fractional
    F2d14(fixed::types::I2F14),

    // Time since 12.00 Jan 1st 1904, UTC
    LDT(i64),

    Tag([u8; 4]),

    // Packed version - major minor - what the actual shit is this
    PVer(u32),

    _USize(usize),
}

impl Display for ValidType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::U8(v) => write!(f, "{v}"),
            Self::I8(v) => write!(f, "{v}"),
            Self::U16(v) => write!(f, "{v}"),
            Self::I16(v) => write!(f, "{v}"),
            Self::U24(v) | Self::U32(v) => write!(f, "{v}"),
            Self::I32(v) => write!(f, "{v}"),
            Self::F16d16(v) => write!(f, "{v}"),
            Self::F2d14(v) => write!(f, "{v}"),
            Self::LDT(v) => {
                const UNIX_DIFF: i64 = 2_082_888_000; // Difference in Seconds between EPOCH and UNIX_EPOCH
                let datetime =
                    chrono::DateTime::from_timestamp(*v + UNIX_DIFF, 0).expect("Invalid Timestamp");
                write!(f, "{datetime}")
            },
            Self::Tag(v) => {
                for c in v {
                    write!(f, "{}", *c as char)?;
                }
                Ok(())
            },
            Self::PVer(_) => unimplemented!(),
            Self::_USize(v) => write!(f, "{v}"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error<IoError> {
    #[error(transparent)]
    IoError(#[from] IoError),

    /// TTF sfntVersion is invalid/unsupported
    #[error("Invalid Version {0:?}")]
    InvalidVersion([u8; 4]),

    /// Parsing error
    #[error("Invalid value at {variable} (Expected {expected}, got {parsed})")]
    Parsing {
        variable: &'static str,
        expected: ValidType,
        parsed:   ValidType,
    },

    /// Allocator failed
    #[error("Allocating {location} failed (expected {expected}, got {allocated}")]
    Allocation {
        location:  &'static str,
        expected:  usize,
        allocated: usize,
    },

    #[error("Unexpected EOF at {0}")]
    UnexpectedEof(usize),

    #[error("Missing required table {missing} to parse {parsing}")]
    MissingTable {
        missing: &'static str,
        parsing: &'static str,
    },
}

pub struct Font<A: core::alloc::Allocator> {
    tables: io::CoreVec<tables::Table<A>, A>,
}

fn verify_header<IoError: core::error::Error>(
    input: &mut impl io::CoreRead<IoError>
) -> Result<(u16, u32), Error<IoError>> {
    let mut read_buf = [0; 12];
    input.read(&mut read_buf[..12])?;

    let mut checksum_full = 0u32;
    for chunk in read_buf[..12].array_chunks() {
        (checksum_full, _) = checksum_full.overflowing_add(u32::from_be_bytes(*chunk));
    }

    // version check
    if read_buf[0..4] != [0x00, 0x01, 0x00, 0x00] {
        return Err(Error::InvalidVersion(
            read_buf[..4]
                .try_into()
                .expect("Failed to make Array (4) from Slice (16)"),
        ));
    }

    let num_tables = u16::from_be_bytes(
        read_buf[4..6]
            .try_into()
            .expect("Failed to make Array (2) from Slice (16)"),
    );
    println!("NumTables {num_tables}");

    let search_range = u16::from_be_bytes(
        read_buf[6..8]
            .try_into()
            .expect("Failed to make Array (2) from Slice (16)"),
    );
    if search_range != 2_u16.pow(num_tables.ilog2()) * 16 {
        return Err(Error::Parsing {
            variable: "SearchRange",
            expected: ValidType::U16(2_u16.pow(num_tables.ilog2()) * 16),
            parsed:   ValidType::U16(search_range),
        });
    }

    let entry_selector = u16::from_be_bytes(
        read_buf[8..10]
            .try_into()
            .expect("Failed to make Array (2) from Slice (16)"),
    );
    if entry_selector != u16::try_from(num_tables.ilog2()).expect("ilog2 downcast failed") {
        return Err(Error::Parsing {
            variable: "EntrySelector",
            expected: ValidType::U32(num_tables.ilog2()),
            parsed:   ValidType::U16(entry_selector),
        });
    }

    let range_shift = u16::from_be_bytes(
        read_buf[10..12]
            .try_into()
            .expect("Failed to make Array (2) from Slice (16)"),
    );
    if range_shift != num_tables * 16 - search_range {
        return Err(Error::Parsing {
            variable: "RangeShift",
            expected: ValidType::U16(num_tables * 16 - search_range),
            parsed:   ValidType::U16(range_shift),
        });
    }

    Ok((num_tables, checksum_full))
}

/// # Panics
/// - If Slice of size `N` is unable to cast to array of type `[u8; N]`
/// - If Downcast fails
pub fn open_font<A: core::alloc::Allocator + Copy, IoError: core::error::Error>(
    allocator: A,
    input: &mut impl io::CoreRead<IoError>,
) -> Result<Font<A>, Error<IoError>> {
    let mut read_buf = [0; 16];

    let (num_tables, mut checksum_full) = verify_header(input)?;

    let mut tables = Vec::with_capacity_in(num_tables as usize, allocator);

    let mut end = 0;
    let mut index = 12;
    for idx in 0..num_tables {
        if input.read(&mut read_buf[..16])? != 16 {
            return Err(Error::Parsing {
                variable: "TableRecords",
                expected: ValidType::U16(num_tables),
                parsed:   ValidType::U16(idx + 1),
            });
        }
        index += 16;

        // Calculate checksum for this region
        for chunk in read_buf[..16].array_chunks() {
            (checksum_full, _) = checksum_full.overflowing_add(u32::from_be_bytes(*chunk));
        }

        let tag: [u8; 4] = read_buf[..4]
            .try_into()
            .expect("Failed to make Array (4) from Slice (16)");

        let checksum = u32::from_be_bytes(
            read_buf[4..8]
                .try_into()
                .expect("Failed to make Array (2) from Slice (16)"),
        );

        let offset = u32::from_be_bytes(
            read_buf[8..12]
                .try_into()
                .expect("Failed to make Array (2) from Slice (16)"),
        );

        let length = u32::from_be_bytes(
            read_buf[12..16]
                .try_into()
                .expect("Failed to make Array (2) from Slice (16)"),
        );

        end = end.min(offset as usize + length as usize);

        tables.push((tag, checksum, offset as usize, length as usize));
    }

    println!("Read {index} bytes");
    let mut parsed_tables = Vec::new_in(allocator);

    let mut checksum_adj = 0;

    tables.sort_by(|(_, _, a, _), (_, _, b, _)| a.cmp(b));
    for (tag, checksum, offset, length) in tables {
        if offset != index {
            println!("Index Mismatch! expected {offset}, got {index}");
            // Fix read bytes

            while index < offset {
                let mut byte = [0; 1];
                if input.read(&mut byte)? != 1 {
                    return Err(Error::UnexpectedEof(index));
                }
                index += 1;
            }
        }

        let mut data_vec = Vec::with_capacity_in(length, allocator);
        if data_vec.capacity() < length {
            return Err(Error::Allocation {
                location:  "TableData",
                expected:  length,
                allocated: data_vec.capacity(),
            });
        }

        // Fill vec as we compute checksum
        let mut checksum_act = 0u32;
        let mut block = [0; 4];

        while index < offset + length {
            if input.read(&mut block)? != 4 {
                return Err(Error::UnexpectedEof(index));
            }

            (checksum_act, _) = checksum_act.overflowing_add(u32::from_be_bytes(block));

            let read_len = 4.min(offset + length - index);
            data_vec.extend_from_slice(&block[..read_len]); // can probably replace with push_within_capacity and remove need for read_len

            index += 4;
        }

        println!("Read {}: {length} at {offset}", ValidType::Tag(tag));

        if tag == *b"head" {
            // find `checksumAdjustment` bytes and subtract from computed checksum
            checksum_adj = u32::from_be_bytes(
                data_vec[8..12]
                    .try_into()
                    .expect("Failed to make Array (4) from Slice (4)"),
            );
            (checksum_act, _) = checksum_act.overflowing_sub(checksum_adj);
        }

        if checksum_act != checksum {
            //println!("Checksum invalid! {checksum} != {checksum_act}");
            return Err(Error::Parsing {
                variable: "TableChecksum",
                expected: ValidType::U32(checksum),
                parsed:   ValidType::U32(checksum_act),
            });
        }

        (checksum_full, _) = checksum_full.overflowing_add(checksum);

        parsed_tables.push(tables::parse_table(
            allocator,
            &parsed_tables,
            tag,
            data_vec,
        )?);
    }

    // ChecksumAdjustment may be set to 0 for version 'OTTO'
    (checksum_full, _) = 0xb1b0afbau32.overflowing_sub(checksum_full);
    if checksum_full != checksum_adj {
        return Err(Error::Parsing {
            variable: "ChecksumAdjustment",
            expected: ValidType::U32(checksum_adj),
            parsed:   ValidType::U32(checksum_full),
        });
    }

    Ok(Font {
        tables: parsed_tables,
    })
}
