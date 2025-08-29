global start
global long_mode_start
extern rust_main

section .text

bits 64
long_mode_start:

    ; initialize segments
    ; setup stack

    call rust_main

    hlt


bits 32    ; By default, GRUB sets us to 32-bit mode.
start:

    ; setup page tables 
    call set_up_page_tables
        
    call enable_paging

    ; switch to long mode 

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    ; jump to long mode 
    jmp gdt64.code:long_mode_start



set_up_page_tables:
    ;
    ; connect pml4 and pml3

    ; write a loop that initializes pml3 to map 4GBs
    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

; Prints `ERR: ` and the given error code to screen and hangs.
; parameter: error code (in ascii) in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64 
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

section .bss
align 4096

p4_table:
    resb 4096
p3_table:
    resb 4096

stack_bottom:
    resb 4096 * 4 ; Reserve this many bytes
stack_top:


