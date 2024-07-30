// Copyright (C) 2024 GLStudios
// SPDX-License-Identifier: LGPL-2.1-only

use core::num::NonZeroU16;

use crate::types::CoreVec;

union SlotContent<T> {
    value:     core::mem::ManuallyDrop<T>,
    next_free: u16,
}

/// ```
/// |       u32        |
/// |  u16   |   u16   |
/// | index  | version |
/// ```
pub type Key = u32;

/// Obviously inspired by the wonderful [Slotmap](https://crates.io/crates/slotmap) crate
pub struct Slotmap<T, A: core::alloc::Allocator = alloc::alloc::Global> {
    slots:     CoreVec<(SlotContent<T>, u16), A>,
    next_free: u16,
    num_elems: u16,
}

pub struct SecondaryMap<T, A: core::alloc::Allocator = alloc::alloc::Global> {
    items:     CoreVec<Option<(T, NonZeroU16)>, A>,
    num_elems: u16,
}

impl<T, A: core::alloc::Allocator> Slotmap<T, A> {
    pub fn new(allocator: A) -> Self {
        let mut slots = CoreVec::with_capacity_in(1, allocator);
        slots.push((SlotContent { next_free: 0 }, 0));

        Self {
            slots,
            next_free: 0,
            num_elems: 0,
        }
    }

    pub fn with_capacity(
        capacity: u16,
        allocator: A,
    ) -> Self {
        let mut slots =
            CoreVec::with_capacity_in(capacity.clamp(1, u16::MAX - 1) as usize, allocator);
        slots.push((SlotContent { next_free: 0 }, 0));

        Self {
            slots,
            next_free: 0,
            num_elems: 0,
        }
    }

    pub fn push(
        &mut self,
        value: T,
    ) -> Key {
        assert!(self.num_elems < u16::MAX - 1, "Slotmap Full");

        let (_, mut avail_ver) = self.slots[self.next_free as usize];
        if avail_ver % 2 == 1 {
            // Push new slot
            self.slots.push((
                SlotContent {
                    value: core::mem::ManuallyDrop::new(value),
                },
                1,
            ));
            let key = u32::from(self.num_elems) << u16::BITS | 1;

            self.next_free = self.num_elems;
            self.num_elems += 1;

            key
        } else {
            avail_ver += 1;
            let index = usize::from(self.next_free);

            self.next_free = unsafe { self.slots[index].0.next_free };
            self.slots[index] = (
                SlotContent {
                    value: core::mem::ManuallyDrop::new(value),
                },
                avail_ver,
            );

            self.num_elems += 1;
            u32::try_from(index).expect("Unexpected truncation") << u16::BITS | u32::from(avail_ver)
        }
    }

    #[inline]
    pub fn contains(
        &self,
        key: Key,
    ) -> bool {
        let version = u16::try_from(key & 0x0000_ffff_u32).expect("Unexpected truncation");
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        !(self.slots.len() < index || self.slots[index].1 != version)
    }

    pub fn get(
        &self,
        key: Key,
    ) -> Option<&T> {
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        if !self.contains(key) {
            return None;
        }

        Some(unsafe { &*self.slots[index].0.value })
    }

    pub fn get_mut(
        &mut self,
        key: Key,
    ) -> Option<&mut T> {
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        if !self.contains(key) {
            return None;
        }

        Some(unsafe { &mut *self.slots[index].0.value })
    }

    pub fn try_pop(
        &mut self,
        key: Key,
    ) -> Option<T> {
        let version = u16::try_from(key & 0x0000_ffff_u32).expect("Unexpected truncation");
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        if !self.contains(key) {
            return None;
        }

        let (value, _) = core::mem::replace(
            &mut self.slots[index],
            (
                SlotContent {
                    next_free: self.next_free,
                },
                version + 1,
            ),
        );

        Some(unsafe { core::mem::ManuallyDrop::into_inner(value.value) })
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots
            .iter()
            .filter(|(_, v)| v % 2 == 1)
            .map(|(v, _)| unsafe { &*v.value })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots
            .iter_mut()
            .filter(|(_, v)| v % 2 == 1)
            .map(|(v, _)| unsafe { &mut *v.value })
    }

    pub fn kv_iter(&self) -> impl Iterator<Item = (Key, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, (_, v))| v % 2 == 1)
            .map(|(i, (t, v))| {
                (
                    u32::try_from(i).expect("Unexpected truncation") << u16::BITS | u32::from(*v),
                    unsafe { &*t.value },
                )
            })
    }
}

impl<T, A: core::alloc::Allocator> Drop for Slotmap<T, A> {
    fn drop(&mut self) {
        for (value, version) in &mut self.slots {
            if *version % 2 == 0 {
                unsafe {
                    core::mem::ManuallyDrop::drop(&mut value.value);
                }
            }
        }
    }
}

impl<T, A: core::alloc::Allocator> SecondaryMap<T, A> {
    pub const fn new(allocator: A) -> Self {
        Self {
            items:     CoreVec::new_in(allocator),
            num_elems: 0,
        }
    }

    pub fn with_capacity(
        capacity: u16,
        allocator: A,
    ) -> Self {
        Self {
            items:     CoreVec::with_capacity_in(
                capacity.clamp(1, u16::MAX - 1) as usize,
                allocator,
            ),
            num_elems: 0,
        }
    }

    #[inline]
    pub fn contains(
        &self,
        key: Key,
    ) -> bool {
        let version = u16::try_from(key & 0x0000_ffff_u32).expect("Unexpected truncation");
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        if let Some((_, prev_ver)) = self.items.get(index).unwrap_or(&None) {
            u16::from(*prev_ver) == version
        } else {
            false
        }
    }

    pub fn insert(
        &mut self,
        key: Key,
        value: T,
    ) {
        let version = u16::try_from(key & 0x0000_ffff_u32).expect("Unexpected truncation");
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        self.items
            .resize_with(usize::from(self.num_elems).max(index + 1), || None);
        self.items[index] = Some((value, unsafe { NonZeroU16::new_unchecked(version | 1) }));
    }

    pub fn get(
        &self,
        key: Key,
    ) -> Option<&T> {
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        if !self.contains(key) {
            return None;
        }

        self.items[index].as_ref().map(|(v, _)| v)
    }

    pub fn get_mut(
        &mut self,
        key: Key,
    ) -> Option<&mut T> {
        let index = usize::try_from(key >> u16::BITS).expect("Failed to cast u16 (u32) to usize");

        if !self.contains(key) {
            return None;
        }

        self.items[index].as_mut().map(|(v, _)| v)
    }
}
