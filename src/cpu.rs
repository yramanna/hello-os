//! The per-CPU data structure.
//!
//! The [`Cpu`] data structure is set as the `GS` base on the CPU.
//! It currently consists of the following:
//!
//! - GDT
//! - TSS
//! - IST stack spaces

use core::arch::asm;
use core::mem::MaybeUninit;
use core::ptr;

use x86::msr;

use crate::gdt::{GlobalDescriptorTable, TaskStateSegment};
use crate::interrupt::x86_xapic::XAPIC;
use crate::thread::SwitchDecision;

const NEW_CPU: Cpu = Cpu::new();

#[repr(C, align(4096))]
pub struct Cpu {

    /// The CPU ID.
    ///
    /// Currently it's the logical APIC ID.
    pub id: usize,

    /// State for the xAPIC driver.
    pub xapic: MaybeUninit<XAPIC>,

    /// The Global Descriptor Table.
    pub gdt: GlobalDescriptorTable,

    /// The Task State Segment.
    pub tss: TaskStateSegment,

}

/// A stack.
#[repr(transparent)]
pub struct Stack<const SZ: usize>([u8; SZ]);

impl<const SZ: usize> Stack<SZ> {
    pub const fn new() -> Self {
        Self([0u8; SZ])
    }

    pub fn bottom(&self) -> *const u8 {
        unsafe {
            (self.0.as_ptr() as *const u8).add(SZ)
        }
    }
}

unsafe impl Send for Cpu {}
unsafe impl Sync for Cpu {}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            // Implement this
        }
    }
}


