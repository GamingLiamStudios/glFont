// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use core::str;

use super::Table;
use crate::{
    types::{
        //CoreBox,
        CoreBox,
        CoreRead,
        CoreVec,
        TrackingReader,
    },
    FontError,
};

pub type ParsedType<A> = Type<A>;

#[derive(Debug, PartialEq, Eq)]
pub enum RecordType {
    Copyright,
    Family,
    Subfamily,
    UniqueIdentifier,
    Full,
    Version,
    PostScript,
    Trademark,
    Manufacturer,
    Designer,
    Description,
    VendorURL,
    DesignerURL,
    License,
    LicenseURL,
    TypographicFamily,
    TypographicSubfamily,
    CompatFull,
    Sample,
    PostScriptCID,
    WWSFamily,
    WWSSubFamily,
    LightPalette,
    DarkPalette,
    PostScriptVariations,
    _Reserved,
    FontSpecific(u16),
}

impl From<u16> for RecordType {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Copyright,
            1 => Self::Family,
            2 => Self::Subfamily,
            3 => Self::UniqueIdentifier,
            4 => Self::Full,
            5 => Self::Version,
            6 => Self::PostScript,
            7 => Self::Trademark,
            8 => Self::Manufacturer,
            9 => Self::Designer,
            10 => Self::Description,
            11 => Self::VendorURL,
            12 => Self::DesignerURL,
            13 => Self::License,
            14 => Self::LicenseURL,
            16 => Self::TypographicFamily,
            17 => Self::TypographicSubfamily,
            18 => Self::CompatFull,
            19 => Self::Sample,
            20 => Self::PostScriptCID,
            21 => Self::WWSFamily,
            22 => Self::WWSSubFamily,
            23 => Self::LightPalette,
            24 => Self::DarkPalette,
            25 => Self::PostScriptVariations,
            15 | 26..256 => Self::_Reserved,
            256..=32767 => Self::FontSpecific(value),
            _ => unreachable!("Invalid Name ID"),
        }
    }
}

#[derive(Debug)]
pub struct Record<A: core::alloc::Allocator> {
    pub name:   RecordType,
    pub string: CoreBox<str, A>,
}

impl<A: core::alloc::Allocator + Copy> Record<A> {
    /// WARNING: Will destructively modify `bytes`
    /// # Panics
    /// - If specified `encoding_id` is utf16 and bytes isn't u16 alligned
    fn from_utf16(
        allocator: A,
        bytes: &mut [u8],
    ) -> CoreBox<str, A> {
        let nibbles: &mut [u16] =
            bytemuck::try_cast_slice_mut(bytes).expect("Invalid input to `Record::from_bytes`");
        if cfg!(target_endian = "little") {
            for v in nibbles.iter_mut() {
                *v = v.swap_bytes();
            }
        }

        // Feel like this can be done slightly more clean
        let char_iter = char::decode_utf16(nibbles.iter().copied())
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER));
        let bytes = char_iter
            .clone()
            .fold(0usize, |size, c| size + c.len_utf8());

        let mut utf8_slices =
            unsafe { CoreBox::new_uninit_slice_in(bytes, allocator).assume_init() };
        let _ = char_iter.fold(0usize, |idx, c| {
            c.encode_utf8(&mut utf8_slices[idx..]);
            idx + c.len_utf8()
        });

        // literally just from_boxed_utf8_unchecked
        unsafe {
            let (ptr, alloc) = CoreBox::into_raw_with_allocator(utf8_slices); // should just be `allocator`
            CoreBox::from_raw_in(ptr as *mut str, alloc)
        }
    }

    pub fn from_bytes(
        allocator: A,
        platform_id: u16,
        encoding_id: u16,
        language_id: u16,
        name: RecordType,
        bytes: &mut [u8],
    ) -> Self {
        Self {
            name,
            string: match (platform_id, encoding_id, language_id) {
                // Unicode
                // TODO: Verify BMP types
                (0, 3..4, _) | (3, 1 | 10, _) => Self::from_utf16(allocator, bytes),
                (..) => panic!("Unrecognised record!"),
            },
        }
    }
}

#[derive(Debug)]
pub struct Type<A: core::alloc::Allocator> {
    pub records: CoreVec<Record<A>, A>,
}

#[tracing::instrument(skip_all, level = "trace")]
pub fn parse_table<A: core::alloc::Allocator + Copy + core::fmt::Debug, R: CoreRead>(
    allocator: A,
    _prev_tables: &[Table<A>],
    reader_actual: &mut R,
) -> Result<Type<A>, FontError<R::IoError>> {
    let mut reader = TrackingReader::new(reader_actual);

    let version: u16 = reader.read_int()?;
    let num_records = reader.read_int::<u16>()? as usize;

    let storage_offset: u16 = reader.read_int()?;
    let mut storage_area_length = usize::MIN;

    // NameRecord
    let mut records_info = CoreVec::with_capacity_in(num_records, allocator);
    for _ in 0..num_records {
        // IDs
        let platform: u16 = reader.read_int()?;
        let encoding: u16 = reader.read_int()?;
        let language: u16 = reader.read_int()?;
        let name: u16 = reader.read_int()?;

        let length: u16 = reader.read_int()?;
        let offset: u16 = reader.read_int()?;

        let begin = offset as usize;
        let end = begin + length as usize;
        records_info.push((platform, encoding, language, name, begin, end));

        storage_area_length = storage_area_length.max(end);
    }

    // TODO: LangTagRecord
    if version == 1 {
        let num_tag_records: u16 = reader.read_int()?;
        for _ in 0..num_tag_records {
            let length: u16 = reader.read_int()?;
            let offset: u16 = reader.read_int()?;
            storage_area_length = storage_area_length.max(offset as usize + length as usize);
        }
    }

    let current_index = reader.finish();
    if storage_offset as usize != current_index {
        tracing::event!(
            tracing::Level::WARN,
            "Read Mismatch! expected {storage_offset}, got {}",
            current_index
        );
        // Fix read bytes
        reader_actual.skip(storage_offset as usize - current_index)?;
    }

    let mut storage_area =
        unsafe { CoreBox::new_uninit_slice_in(storage_area_length, allocator).assume_init() };
    let read = reader_actual.read(&mut storage_area)?;
    if read < storage_area_length {
        return Err(FontError::UnexpectedEop {
            location: "name::storage_area",
            needed:   storage_area_length - read,
        });
    }

    let mut records = CoreVec::with_capacity_in(num_records, allocator);
    for (platform_id, encoding_id, language_id, name_id, begin, end) in records_info {
        records.push(Record::from_bytes(
            allocator,
            platform_id,
            encoding_id,
            language_id,
            name_id.into(),
            &mut storage_area[begin..end],
        ));
    }

    //println!("{records:#?}");

    Ok(Type { records })
}
