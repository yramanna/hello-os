//! Interrupt handling.

// Copyright 2021 Zhaofeng Li
// Copyright 2017 Philipp Oppermann
//
// Licensed under the MIT license <http://opensource.org/licenses/MIT>.
// See top-level LICENSE.

mod exception;
mod idt;
mod ioapic;
mod lapic;
mod mps;
pub mod x86_xapic;

use core::arch::{asm, naked_asm};
use core::convert::{Into, TryFrom};

use bit_field::BitField;
use x86::Ring;
use x86::bits64::paging::VAddr;
use x86::io::{inb, outb};

use crate::boot::spin_forever;
pub use exception::Exception;
use exception::EXCEPTION_MAX;
use idt::Idt;
pub use lapic::{boot_ap, end_of_interrupt, set_timer};
use verified::trap::Registers;

/// The IRQ offset.
pub const IRQ_OFFSET: usize = 32;

pub const IRQ_TIMER: usize = 0;
pub const IRQ_IOMMU_FAULT: usize = 1;

pub const IST_EXCEPTION: usize = 1;
pub const IST_IRQ: usize = 2;

/// The global IDT.
static mut GLOBAL_IDT: Idt = Idt::new();

const PIC1_DATA: u16 = 0x21;
const PIC2_DATA: u16 = 0xa1;

/// An amount of cycles.
#[derive(Debug)]
#[repr(transparent)]
pub struct Cycles(pub usize);

#[repr(C)]
struct TrampolineMarker(());

macro_rules! wrap_interrupt_with_error_code {
    ($handler:path) => {{
        let _: unsafe extern "C" fn(&mut Registers) = $handler;

        /// Interrupt trampoline
        #[naked]
        unsafe extern "C" fn trampoline(_: TrampolineMarker) {
            // Figure 6-7. Stack Usage on Transfers to Interrupt and Exception Handling Routines

            // Here rsp is at an InterruptStackFrame
            // [rip][cs][eflags][esp][ss]
            naked_asm!(

                "cld",
                "push rax",
                "push rdi",
                "push rsi",
                "push rdx",
                "push rcx",

                // other registers here

                // fn handler(registers: &mut Registers)
                "mov rdi, rsp",
                "call {handler}",

                // ... restore other regs
                
                "pop rcx",
                "pop rdx",
                "pop rsi",
                "pop rdi",
                "pop rax",

                "iretq",

                //breakpoint = sym crate::debugger::breakpoint,
                handler = sym $handler,
            );
        }

        trampoline
    }}
}

macro_rules! wrap_interrupt {
    ($handler:path) => {{
        let _: unsafe extern "C" fn(&mut Registers) = $handler;

        /// Interrupt trampoline
        #[naked]
        unsafe extern "C" fn trampoline(_: TrampolineMarker) {
            // Figure 6-7. Stack Usage on Transfers to Interrupt and Exception Handling Routines

            // Here rsp is at an InterruptStackFrame
            // [rip][cs][eflags][esp][ss]
            naked_asm!(
                //"call {breakpoint}",

                "cld",

                "push 0", // error_code

                "push rax",
                "push rdi",
                "push rsi",
                "push rdx",
                "push rcx",

                // ... same as above

                // fn handler(registers: &mut Registers)
                "mov rdi, rsp",
                "call {handler}",

                // .. don't forget
                "pop rcx",
                "pop rdx",
                "pop rsi",
                "pop rdi",
                "pop rax",

                "add rsp, 8", // error_code

                "iretq",

                //breakpoint = sym crate::debugger::breakpoint,
                handler = sym $handler,
            );
        }

        trampoline
    }}
}

// "x86-interrupt" is gated behind #![feature(abi_x86_interrupt)].

/// A handler function for an interrupt or an exception without error code.
pub type HandlerFunc = unsafe extern "C" fn(&mut PtRegs);

pub type HandlerFuncWithErrCode = unsafe extern "C" fn(&mut PtRegs);

/// Invalid Opcode handler.
unsafe extern "C" fn invalid_opcode(regs: &mut PtRegs) {
    log::error!("CPU {}, Invalid Opcode: {:#x?}", crate::cpu::get_cpu_id(), regs);
    //crate::debugger::breakpoint(2);
    spin_forever();
}


/// Registers passed to the ISR.
#[repr(C)]
#[derive(Debug)]
pub struct PtRegs {
    // add the 5 things + error code added by the hardware

    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rax: u64,
}


/// Initializes global interrupt controllers.
///
/// This should be called only once.
pub unsafe fn init() {
    let pic1 = inb(PIC1_DATA);
    let pic2 = inb(PIC2_DATA);

    log::debug!("PIC masks: PIC1={:#x?}, PIC2={:#x?}", pic1, pic2);

    // Disable 8259 PIC
    outb(PIC1_DATA, 0xff);
    outb(PIC2_DATA, 0xff);

    let idt = &mut GLOBAL_IDT;

    // initialize idt with handlers 
    //

    idt.breakpoint.set_handler_fn(wrap_interrupt!(breakpoint));
   
    idt.page_fault.set_handler_fn(wrap_interrupt_with_error_code!(page_fault));

    idt.interrupts[IRQ_TIMER].set_handler_fn(wrap_interrupt!(timer));

    let ioapic_base = mps::probe_ioapic();
    ioapic::init(ioapic_base);
}

/// Initializes per-CPU interrupt controllers.
///
/// This should be called only once per CPU.
pub unsafe fn init_cpu() {
    lapic::init();
    ioapic::init_cpu();

    GLOBAL_IDT.load();

    asm!("sti");
}
