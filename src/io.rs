// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only
pub trait CoreRead<E: core::error::Error> {
    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, E>;
}

impl<T: std::io::Read> CoreRead<std::io::Error> for T {
    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, std::io::Error> {
        self.read(buf)
    }
}

// TODO: Custom no_std Vec type
pub type CoreVec<T, A> = std::vec::Vec<T, A>;
