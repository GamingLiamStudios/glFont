// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(allocator_api)] // no_std
#![feature(generic_const_exprs)] // CoreRead::read_int
#![feature(new_range_api)] // glyf
#![feature(new_uninit)] // name
#![allow(incomplete_features)]

extern crate alloc;

mod font;
mod tables;
mod types;

pub use font::{
    open_font,
    Collection as FontCollection,
    Font,
    Trait as FontTrait,
};
pub use tables::name::RecordType as NameRecord;
pub use types::Error as FontError;
