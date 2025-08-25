#![deny(unused_must_use)]

macro_rules! source {
    ($($arg:tt)*) => {{
        println!("cargo:rerun-if-changed={}", format_args!($($arg)*));
        let path = format!($($arg)*);
        path
    }};
}

fn main() {
    source!("src/linker.ld");
    add_x86_64_asm("boot.asm");
    add_x86_64_asm("multiboot_header.asm");
}

fn add_x86_64_asm(source: &str) {
    let mut mb = nasm_rs::Build::new();
    mb.file(&source!("src/{}", source));
    mb.target("");
    mb.flag("-felf64");

    let objects = mb.compile_objects().unwrap();
    for object in objects {
        println!("cargo:rustc-link-arg={}", object.to_str().unwrap());
    }
}
