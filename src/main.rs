#![cfg_attr(not(test), no_std, no_main)]
#![allow(static_mut_refs)]

mod cpu;
mod error;
mod gdt;
mod interrupt;

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    println!("Hello from Rust!");
    loop {}
}
