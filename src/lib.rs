// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
//#![feature(const_mut_refs)]
#![feature(allocator_api)]
#![feature(array_chunks)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(generic_const_exprs)]
#![allow(clippy::missing_errors_doc)]

use std::fmt::Display;

use io::CoreRead;

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

    // Packed version - 16 major, 16 minor
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
                    chrono::DateTime::from_timestamp(*v - UNIX_DIFF, 0).expect("Invalid Timestamp");
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
pub enum Error<IoError: core::fmt::Debug> {
    #[error(transparent)]
    IoError(#[from] io::CoreReadError<IoError>),

    /// TTF sfntVersion is invalid/unsupported
    #[error("Invalid Sfnt Version {0:?}")]
    InvalidSfntVersion([u8; 4]),

    /// Parsing error
    #[error("Invalid value at {variable} (Expected {expected}, got {parsed})")]
    Parsing {
        variable: &'static str,
        expected: ValidType,
        parsed:   ValidType,
    },

    #[error("Invalid tag {0:?}")]
    InvalidTag([u8; 4]),

    #[error("Invalid version at {location} (got {version})")]
    InvalidVersion {
        location: &'static str,
        version:  u32,
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

    #[error("Unexpected EOP in {location} (needed {needed})")]
    UnexpectedEop {
        location: &'static str,
        needed:   usize,
    },

    #[error("Missing required table {missing} to parse {parsing}")]
    MissingTable {
        missing: &'static str,
        parsing: &'static str,
    },
}

pub struct Font<A: core::alloc::Allocator + core::fmt::Debug> {
    tables: io::CoreVec<tables::Table<A>, A>,
}

fn verify_header<R: io::CoreRead>(input: &mut R) -> Result<u16, Error<R::IoError>> {
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
            expected: ValidType::U16(2_u16.pow(num_tables.ilog2()) * 16),
            parsed:   ValidType::U16(search_range),
        });
    }

    let entry_selector: u16 = input.read_int()?;
    if entry_selector != u16::try_from(num_tables.ilog2()).expect("ilog2 downcast failed") {
        return Err(Error::Parsing {
            variable: "EntrySelector",
            expected: ValidType::U32(num_tables.ilog2()),
            parsed:   ValidType::U16(entry_selector),
        });
    }

    let range_shift: u16 = input.read_int()?;
    if range_shift != num_tables * 16 - search_range {
        return Err(Error::Parsing {
            variable: "RangeShift",
            expected: ValidType::U16(num_tables * 16 - search_range),
            parsed:   ValidType::U16(range_shift),
        });
    }

    Ok(num_tables)
}

/// # Panics
/// - If Slice of size `N` is unable to cast to array of type `[u8; N]`
/// - If Downcast fails
#[tracing::instrument(level = "trace", skip_all)]
pub fn open_font<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: io::CoreRead>(
    allocator: A,
    input: &mut R,
) -> Result<Font<A>, Error<R::IoError>> {
    let mut reader = io::ChecksumReader::new(input);

    let num_tables = verify_header(&mut reader)?;
    let mut tables = Vec::with_capacity_in(num_tables as usize, allocator);

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

        let mut tag_reader = io::ChecksumReader::new(&mut reader);

        tracing::event!(
            tracing::Level::TRACE,
            "Read {}: {length} at {offset}",
            ValidType::Tag(tag)
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
        } else {
            if parsed.is_ok() {
                parsed_tables.push(parsed?);
            } else {
                let error = parsed.unwrap_err();
                if !matches!(error, Error::InvalidTag(_)) {
                    return Err(error);
                };
            }
        }

        if checksum_act != checksum {
            println!("Checksum invalid! {checksum} != {checksum_act}");
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
            expected: ValidType::U32(checksum_adj),
            parsed:   ValidType::U32(checksum),
        });
    }

    Ok(Font {
        tables: parsed_tables,
    })
}
