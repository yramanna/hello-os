#![cfg_attr(not(test), no_std, no_main)]
#![allow(static_mut_refs)]

mod cpu;
mod error;
mod gdt;
mod interrupt;

use core::panic::PanicInfo;

mod serial;

// Add println! macro that redirects to serial
#[macro_export]
macro_rules! println {
    () => ($crate::serial_println!());
    ($($arg:tt)*) => ($crate::serial_println!($($arg)*));
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    println!("Hello from Rust!");
    loop {}
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
