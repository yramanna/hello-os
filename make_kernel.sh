#!/bin/bash

KERNEL="build/hello-os"

echo "=== Checking Kernel Binary ==="
echo

if [ ! -f "$KERNEL" ]; then
    echo "ERROR: Kernel file not found at $KERNEL"
    echo "Run 'make build' first"
    exit 1
fi

echo "✓ Kernel file exists"
echo

echo "=== ELF Header ==="
readelf -h "$KERNEL" | grep -E "Type:|Entry|Machine:"
echo

echo "=== Checking for Multiboot Header ==="
if hexdump -C "$KERNEL" | head -n 50 | grep -q "d6 50 52 e8"; then
    echo "✓ Multiboot2 magic number found!"
    hexdump -C "$KERNEL" | grep "d6 50 52 e8" | head -n 1
else
    echo "✗ WARNING: Multiboot2 magic number NOT found!"
    echo "  The kernel may not boot properly."
fi
echo

echo "=== Sections ==="
objdump -h "$KERNEL" | grep -E "multiboot|\.text|\.rodata|\.data|\.bss"
echo

echo "=== Entry Point ==="
objdump -f "$KERNEL" | grep "start address"
echo

echo "=== First 256 bytes of kernel (hex) ==="
hexdump -C "$KERNEL" | head -n 16
echo

echo "=== Symbol Table (first 20) ==="
nm "$KERNEL" | head -n 20
echo

echo "=== Try running with: ==="
echo "  qemu-system-x86_64 -kernel $KERNEL -serial stdio -nographic"
echo "  OR"
echo "  qemu-system-x86_64 -kernel $KERNEL -serial stdio -display none"