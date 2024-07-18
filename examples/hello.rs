// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
#![feature(allocator_api)]

use std::{
    alloc::Global,
    error::Error,
    fs,
};

fn main() -> Result<(), Box<dyn Error>> {
    let fmt_subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(fmt_subscriber)?;

    let mut font_file = fs::File::open("NotoSerif-Regular.ttf")?;
    let _font = glfont::open_font(Global, &mut font_file)?;

    println!("Hello World!");

    Ok(())
}
