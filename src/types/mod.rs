// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
mod io;
mod slotmap;

pub use io::*;
pub use slotmap::{
    Key as SlotmapKey,
    //SecondaryMap,
    Slotmap,
};

// In case we decide to not use the default Vec impl (for whatever reason)
pub type CoreVec<T, A> = alloc::vec::Vec<T, A>;
pub type CoreBox<T, A> = alloc::boxed::Box<T, A>;

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
    Ldt(i64),

    Tag([u8; 4]),

    // Packed version - 16 major, 16 minor
    PVer(u32),

    _USize(usize),
}

impl core::fmt::Display for ValidType {
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        match self {
            Self::U8(v) => write!(f, "{v}"),
            Self::I8(v) => write!(f, "{v}"),
            Self::U16(v) => write!(f, "{v}"),
            Self::I16(v) => write!(f, "{v}"),
            Self::U24(v) | Self::U32(v) => write!(f, "{v}"),
            Self::I32(v) => write!(f, "{v}"),
            Self::F16d16(v) => write!(f, "{v}"),
            Self::F2d14(v) => write!(f, "{v}"),
            Self::Ldt(v) => {
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
    Io(#[from] CoreReadError<IoError>),

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
