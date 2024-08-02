// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
#![feature(allocator_api)]

use std::{
    alloc,
    error::Error,
    fs,
};

use glfont::{
    render::SubPixelAlignment,
    render_to_buf,
    FontCollection,
    FontTrait,
};

// Would use WM supplied info to calc this in actual use
const DPI: u16 = 72;

fn main() -> Result<(), Box<dyn Error>> {
    let fmt_subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(fmt_subscriber)?;

    let mut fonts = FontCollection::new(alloc::Global);

    let mut font_file = fs::File::open("JetBrainsMono-Bold.ttf")?;
    let font = fonts.add_loaded(glfont::open_font(alloc::Global, &mut font_file)?);

    println!("Hello World!");
    println!("Font id is {:?}", fonts.get(font).id());

    let formatted = [glfont::render::FormattedSlice {
        id:   font,
        size: 48,
        text: "fuck off",
    }];

    let mut display_buf = [rgb::Gray::<u8>::new(u8::MAX); 512 * 342];
    render_to_buf(
        &fonts,
        &formatted,
        &mut display_buf,
        512,
        DPI,
        SubPixelAlignment::None,
    )?;

    let path = std::path::Path::new("./line.png");
    let file = std::fs::File::create(path)?;

    let mut png_enc = png::Encoder::new(std::io::BufWriter::new(file), 512, 342);
    png_enc.set_color(png::ColorType::Grayscale);
    png_enc.set_depth(png::BitDepth::Eight);

    let mut writer = png_enc.write_header().expect("Failed to write png header");
    writer.write_image_data(bytemuck::cast_slice(&display_buf))?;

    Ok(())
}
