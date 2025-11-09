global start
global long_mode_start
global _bootinfo
extern rust_main

section .data
; Storage for multiboot info pointer
_bootinfo:
    dq 0

section .text

bits 64
long_mode_start:

    ; initialize segments
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; setup stack
    mov rsp, stack_top

    call rust_main

    hlt

bits 32    ; By default, GRUB sets us to 32-bit mode.
start:
    ; Save multiboot info pointer from EBX
    ; EAX contains the magic number, EBX contains the multiboot info pointer
    call check_multiboot
    
    ; Move stack pointer to our stack
    mov esp, stack_top

    ; setup page tables 
    call set_up_page_tables
        
    call enable_paging

    ; switch to long mode 

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    ; jump to long mode 
    jmp gdt64.code:long_mode_start

check_multiboot:
    ; Check if EAX contains the multiboot2 magic number
    cmp eax, 0x36d76289
    jne .no_multiboot
    
    ; Save the multiboot info pointer from EBX
    ; We need to save it for later use in Rust
    mov dword [_bootinfo], ebx
    mov dword [_bootinfo + 4], 0
    
    ret

.no_multiboot:
    mov al, 'M'
    jmp error

set_up_page_tables:
    ; Clear the page tables
    mov edi, p4_table
    mov ecx, 4096
    xor eax, eax
    rep stosd

    ; Map first P4 entry to P3 table
    mov eax, p3_table
    or eax, 0b11 ; present + writable
    mov [p4_table], eax

    ; Map first 4GB in P3 table using 1GB huge pages
    mov edi, p3_table
    mov eax, 0b10000011 ; present + writable + huge page (1GB)
    mov ecx, 4 ; Map 4 entries (4GB)

.map_p3_table:
    mov [edi], eax
    add eax, 0x40000000 ; Add 1GB
    add edi, 8
    loop .map_p3_table

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