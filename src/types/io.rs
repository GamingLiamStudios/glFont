// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

#[derive(thiserror::Error, Debug)]
pub enum CoreReadError<IoError: core::fmt::Debug> {
    #[error(transparent)]
    Io(#[from] IoError),

    #[error("Expected {0} more bytes")]
    UnexpectedEnd(usize),
}

pub trait CoreRead {
    type IoError: core::error::Error;

    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, CoreReadError<Self::IoError>>;

    fn skip(
        &mut self,
        skip: usize,
    ) -> Result<usize, CoreReadError<Self::IoError>> {
        let mut buf = [0u8; 1];

        let mut total = 0;
        while total != skip {
            let read = self.read(&mut buf)?;
            total += read;

            if read != buf.len() {
                break;
            }
        }

        Ok(total)
    }

    fn read_int<T: num_traits::PrimInt + bytemuck::AnyBitPattern>(
        &mut self
    ) -> Result<T, CoreReadError<Self::IoError>>
    where
        [(); size_of::<T>()]:,
    {
        let mut bytes = [0u8; size_of::<T>()];
        let read = self.read(&mut bytes)?;
        if read == bytes.len() {
            Ok(T::to_be(*bytemuck::from_bytes(&bytes)))
        } else {
            Err(CoreReadError::UnexpectedEnd(bytes.len() - read))
        }
    }
}

impl<T: std::io::Read> CoreRead for T {
    type IoError = std::io::Error;

    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, CoreReadError<Self::IoError>> {
        self.read(buf).map_err(CoreReadError::Io)
    }
}

pub struct TrackingReader<'a, R: CoreRead> {
    reader: &'a mut R,
    index:  usize,
}

impl<'a, R: CoreRead> TrackingReader<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Self { reader, index: 0 }
    }

    pub const fn finish(self) -> usize {
        self.index
    }

    pub const fn total_read(&self) -> usize {
        self.index
    }
}

impl<R: CoreRead> CoreRead for TrackingReader<'_, R> {
    type IoError = R::IoError;

    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, CoreReadError<Self::IoError>> {
        self.reader.read(buf).inspect(|read| self.index += read)
    }
}

pub struct ChecksumReader<'a, R: CoreRead> {
    reader: &'a mut R,
    index:  usize,

    checksum: u32,
    next_add: u32,
}

impl<'a, R: CoreRead> ChecksumReader<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Self {
            reader,
            index: 0,
            checksum: 0,
            next_add: 0,
        }
    }

    /// NOTE: Assumes that the reader has a length of a multiple of 4
    pub fn finish(mut self) -> Result<u32, CoreReadError<R::IoError>> {
        let mut garbage = [0; 3];

        // Should only be 0
        let remain = self.index.next_multiple_of(4) - self.index;
        self.read(&mut garbage[..remain])?;

        for value in garbage.into_iter().take(remain) {
            self.next_add <<= 8;
            self.next_add |= u32::from(value);
        }

        if remain != 0 {
            (self.checksum, _) = self.checksum.overflowing_add(self.next_add);
        }

        Ok(self.checksum)
    }

    pub const fn total_read(&self) -> usize {
        self.index
    }
}

impl<R: CoreRead> CoreRead for ChecksumReader<'_, R> {
    type IoError = R::IoError;

    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, CoreReadError<Self::IoError>> {
        self.reader.read(buf).inspect(|read| {
            let mut index = 0;
            while index != *read {
                self.next_add <<= 8;
                self.next_add |= u32::from(buf[index]);
                index += 1;

                if (self.index + index) % 4 == 0 {
                    (self.checksum, _) = self.checksum.overflowing_add(self.next_add);
                }
            }

            self.index += index;
        })
    }
}
