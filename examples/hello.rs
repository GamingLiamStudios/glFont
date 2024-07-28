// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
#![feature(allocator_api)]

use std::{
    alloc,
    error::Error,
    fs,
};

use glfont::FontTrait;

fn main() -> Result<(), Box<dyn Error>> {
    let fmt_subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(fmt_subscriber)?;

    let mut font_file = fs::File::open("NotoSerif-Regular.ttf")?;
    let font = glfont::open_font(alloc::Global, &mut font_file)?;

    println!("Hello World!");
    println!(
        "Font name is {}",
        font.name_record(glfont::NameRecord::Full)
            .unwrap_or("Unknown")
    );

    Ok(())
}
