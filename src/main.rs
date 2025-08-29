#![cfg_attr(not(test), no_std, no_main)]

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    println!("Hello from Rust!");

    loop {}
}


