#![cfg_attr(not(test), no_std, no_main)]
#![allow(static_mut_refs)] 

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    println!("Hello from Rust!");
    loop {}
}