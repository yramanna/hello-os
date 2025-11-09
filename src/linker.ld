ENTRY(start)

SECTIONS {
  . = 0x100000; /* Tells GRUB to load the kernel starting at the 1MiB mark */

  .boot :
  {
    KEEP(*(.multiboot_header))
    . = ALIGN(4K);
  }
  .rodata :
  {
    /* ensure that the multiboot header is at the beginning */
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .text :
  {
    *(.text .text.*)
    . = ALIGN(4K);
  }

  .data :
  {
    *(.data .data.*)
    . = ALIGN(4K);
  }

  .bss :
  {
    *(.bss .bss.*)
    . = ALIGN(4K);
  }

  .got :
  {
    *(.got)
    . = ALIGN(4K);
  }

  .got.plt :
  {
    *(.got.plt)
    . = ALIGN(4K);
  }

  .data.rel.ro : ALIGN(4K) {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    . = ALIGN(4K);
  }

  .gcc_except_table : ALIGN(4K) {
    *(.gcc_except_table)
    . = ALIGN(4K);
  }

  /* Mark the end of the kernel */
  . = ALIGN(4K);
  __end = .;
}