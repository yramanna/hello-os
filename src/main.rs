#![cfg_attr(not(test), no_std, no_main)]
#![allow(static_mut_refs)]
#![feature(alloc_error_handler)]

mod cpu;
mod error;
mod gdt;
mod interrupt;
mod serial;
mod memory;

use core::panic::PanicInfo;

#[macro_use]
extern crate lazy_static;

extern crate alloc;

// Add println! macro that redirects to serial
#[macro_export]
macro_rules! println {
    () => ($crate::serial::_print(format_args!("\n")));
    ($($arg:tt)*) => ({
        $crate::serial::_print(format_args!($($arg)*));
        $crate::serial::_print(format_args!("\n"));
    });
}

// Reference to the multiboot info pointer saved in boot.asm
extern "C" {
    static _bootinfo: usize;
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    unsafe {
        println!("\n=== Kernel Starting ===");
        
        // Check if we can read/write to see CPU state
        let rflags: u64;
        core::arch::asm!("pushfq; pop {}", out(reg) rflags);
        println!("RFLAGS: {:#x}", rflags);
        
        // Initialize GDT and TSS
        println!("Initializing GDT...");
        gdt::init_cpu();
        
        // Initialize memory allocator BEFORE enabling interrupts
        // This must come early since interrupt handlers might allocate
        println!("Initializing memory allocator...");
        let boot_info_addr = _bootinfo;
        println!("Multiboot info at: {:#x}", boot_info_addr);
        mem::init(boot_info_addr);
        
        // Initialize interrupt controllers and IDT
        println!("Initializing interrupts...");
        interrupt::init();
        
        println!("Initializing per-CPU interrupt state...");
        interrupt::init_cpu();
        
        println!("\n=== Kernel Initialized Successfully ===\n");
        
        // Test the allocator
        test_allocator();
        
        // Infinite loop - timer interrupts will fire and print dots
        loop {
            core::arch::asm!("hlt");
        }
    }
}

/// Test the memory allocator
fn test_allocator() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    
    println!("\n=== Testing Memory Allocator ===");
    
    // Test Box allocation
    println!("Testing Box<u64> allocation...");
    let boxed_value = Box::new(42u64);
    println!("Allocated Box with value: {}", *boxed_value);
    
    // Test Vec allocation
    println!("Testing Vec allocation...");
    let mut vec = Vec::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);
    println!("Created Vec with {} elements: {:?}", vec.len(), vec);
    
    // Test larger allocation
    println!("Testing larger Box allocation...");
    let large_box = Box::new([0u8; 1024]);
    println!("Allocated large Box of {} bytes", large_box.len());
    
    println!("=== Allocator Tests Passed ===\n");
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n!!! KERNEL PANIC !!!");
    println!("{}", info);
    
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

/// Allocation error handler
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
