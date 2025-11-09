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
use idt::Idt;
use x86::io::{inb, outb};

//pub use lapic::{boot_ap, end_of_interrupt, set_timer};

/// The IRQ offset.
pub const IRQ_OFFSET: usize = 32;
pub const IRQ_TIMER: usize = 0;

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

#[repr(C)]
struct TrampolineMarkerErrorCode(());

macro_rules! wrap_interrupt_with_error_code {
    ($handler:path) => {{
        let _: unsafe extern "C" fn(&mut InterruptStackFrame) = $handler;

        /// Interrupt trampoline
        #[unsafe(naked)]
        unsafe extern "C" fn trampoline(_: TrampolineMarkerErrorCode) {
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

                // push missing registers
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "push rbx",
                "push rbp",
                "push r12",
                "push r13",
                "push r14",
                "push r15",

                // fn handler(registers: &mut InterruptStackFrame)
                "mov rdi, rsp",
                "call {handler}",

                // pop missing registers
                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop rbp",
                "pop rbx",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rcx",
                "pop rdx",
                "pop rsi",
                "pop rdi",
                "pop rax",
                "add rsp, 8",  // pop error code

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
        let _: unsafe extern "C" fn(&mut InterruptStackFrame) = $handler;

        /// Interrupt trampoline
        #[unsafe(naked)]
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
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "push rbx",
                "push rbp",
                "push r12",
                "push r13",
                "push r14",
                "push r15",

                // fn handler(registers: &mut InterruptStackFrame)
                "mov rdi, rsp",
                "call {handler}",

                // .. don't forget
                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop rbp",
                "pop rbx",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
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

pub type HandlerFuncWithErrCode = unsafe extern "C" fn(_: TrampolineMarkerErrorCode);
pub type HandlerFunc = unsafe extern "C" fn(_: TrampolineMarker);

/// Just as an example: Invalid Opcode handler.
unsafe extern "C" fn invalid_opcode(regs: &mut InterruptStackFrame) {}

/// Page Fault handler.
unsafe extern "C" fn page_fault(regs: &mut InterruptStackFrame) {
    let cr2: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2);
    }
    panic!("Page fault at address {:#x}, RIP: {:#x}, error code: {:#x}",
           cr2, regs.rip, regs.error_code);
}

/// General Protection Fault handler.
unsafe extern "C" fn general_protection_fault(regs: &mut InterruptStackFrame) {
    panic!("General Protection Fault at RIP: {:#x}, error code: {:#x}",
           regs.rip, regs.error_code);
}

/// Double Fault handler.
unsafe extern "C" fn double_fault(regs: &mut InterruptStackFrame) {
    panic!("Double Fault at RIP: {:#x}", regs.rip);
}

/// Breakpoint handler.
unsafe extern "C" fn breakpoint(regs: &mut InterruptStackFrame) {
}

/// Timer interrupt handler.
unsafe extern "C" fn timer(regs: &mut InterruptStackFrame) {
    use crate::interrupt::{lapic, Cycles};
    lapic::set_timer(Cycles(100_000)); 
    // Print a dot for each timer interrupt
    use x86::io::outb;
    const SERIAL_PORT: u16 = 0x3f8;
    unsafe {
        outb(SERIAL_PORT, b'.');
    }
    
    // Acknowledge the interrupt
    lapic::end_of_interrupt();
}

/// Registers passed to the interrupt handler
#[repr(C)]
#[derive(Debug)]
pub struct InterruptStackFrame {
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
    // Implement: add the 5 values + error code added by the hardware
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}   

/// Initializes global interrupt controllers.
///
/// This should be called only once
#[allow(static_mut_refs)]
pub unsafe fn init() {
    unsafe {
        let pic1 = inb(PIC1_DATA);
        let pic2 = inb(PIC2_DATA);
        // Disable 8259 PIC
        outb(PIC1_DATA, 0xff);
        outb(PIC2_DATA, 0xff);

        let idt = &mut GLOBAL_IDT;

        // Implement:
        //
        // You need to initialize idt with handlers similar to a couple of examples below
        // of course you need handler implementations, check invalid_opcode above
        // idt.breakpoint.set_handler_fn(wrap_interrupt!(breakpoint));
        // idt.page_fault.set_handler_fn(wrap_interrupt_with_error_code!(page_fault));
        // idt.interrupts[IRQ_TIMER].set_handler_fn(wrap_interrupt!(timer));
        
        // Set up exception handlers
        idt.divide_by_zero.set_handler_fn(wrap_interrupt!(invalid_opcode));
        idt.breakpoint.set_handler_fn(wrap_interrupt!(breakpoint));
        idt.invalid_opcode.set_handler_fn(wrap_interrupt!(invalid_opcode));
        idt.double_fault.set_handler_fn(wrap_interrupt_with_error_code!(double_fault));
        idt.general_protection_fault.set_handler_fn(wrap_interrupt_with_error_code!(general_protection_fault));
        idt.page_fault.set_handler_fn(wrap_interrupt_with_error_code!(page_fault));
        
        // Set up timer interrupt handler
        idt.interrupts[IRQ_TIMER].set_handler_fn(wrap_interrupt!(timer));

        let ioapic_base = mps::probe_ioapic();
        ioapic::init(ioapic_base);
    }
}

/// Initializes per-CPU interrupt controllers.
///
/// This should be called only once per CPU.
pub unsafe fn init_cpu() {
    unsafe {
        lapic::init();
        ioapic::init_cpu();
        GLOBAL_IDT.load();

        asm!("sti");
    }
}
