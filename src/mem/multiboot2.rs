//! Multiboot2 boot information parser

use core::mem;
use core::slice;

const MULTIBOOT2_TAG_TYPE_END: u32 = 0;
const MULTIBOOT2_TAG_TYPE_MMAP: u32 = 6;

/// Boot information structure passed by GRUB
#[repr(C)]
pub struct BootInfo {
    total_size: u32,
    _reserved: u32,
}

impl BootInfo {
    /// Parse the boot information structure
    /// 
    /// # Safety
    /// The pointer must point to valid multiboot2 data
    pub unsafe fn parse(ptr: *const u8) -> Option<&'static Self> {
        if ptr.is_null() {
            return None;
        }
        Some(&*(ptr as *const BootInfo))
    }

    /// Get the memory map tag
    pub fn memory_map_tag(&self) -> Option<&MemoryMapTag> {
        self.find_tag(MULTIBOOT2_TAG_TYPE_MMAP)
    }

    /// Find a tag by type
    fn find_tag<T>(&self, tag_type: u32) -> Option<&T> {
        let self_ptr = self as *const BootInfo as usize;
        let mut current = self_ptr + 8; // Skip total_size and reserved

        loop {
            let tag = unsafe { &*(current as *const TagHeader) };

            if tag.typ == MULTIBOOT2_TAG_TYPE_END {
                return None;
            }

            if tag.typ == tag_type {
                return Some(unsafe { &*(current as *const T) });
            }

            // Move to next tag (8-byte aligned)
            current = (current + tag.size as usize + 7) & !7;
        }
    }
}

/// Common header for all tags
#[repr(C)]
struct TagHeader {
    typ: u32,
    size: u32,
}

/// Memory map tag
#[repr(C)]
pub struct MemoryMapTag {
    typ: u32,
    size: u32,
    entry_size: u32,
    entry_version: u32,
}

/// Memory map entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryArea {
    pub base_addr: u64,
    pub length: u64,
    pub typ: u32,
    _reserved: u32,
}

impl MemoryMapTag {
    /// Get an iterator over memory areas
    pub fn memory_areas(&self) -> MemoryAreaIter {
        let self_ptr = self as *const MemoryMapTag;
        let start = unsafe { self_ptr.add(1) } as usize;
        let end = self_ptr as usize + self.size as usize;
        let entry_size = self.entry_size as usize;

        MemoryAreaIter {
            current: start,
            end,
            entry_size,
        }
    }
}

/// Iterator over memory areas
pub struct MemoryAreaIter {
    current: usize,
    end: usize,
    entry_size: usize,
}

impl Iterator for MemoryAreaIter {
    type Item = MemoryArea;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        let area = unsafe { *(self.current as *const MemoryArea) };
        self.current += self.entry_size;

        Some(area)
    }
}