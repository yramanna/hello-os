#!/bin/bash

set -e

echo "Creating bootable ISO with GRUB..."

# Clean up old iso directory
rm -rf isodir
rm -f hello-os.iso

# Create directory structure
mkdir -p isodir/boot/grub

# Copy kernel
cp build/hello-os isodir/boot/hello-os

# Create GRUB config
cat > isodir/boot/grub/grub.cfg << 'GRUBEOF'
set timeout=0
set default=0

menuentry "Hello OS" {
    multiboot2 /boot/hello-os
    boot
}
GRUBEOF

echo "Building ISO..."
# Create ISO with GRUB - try different command names
if command -v grub-mkrescue &> /dev/null; then
    grub-mkrescue -o hello-os.iso isodir
    echo "✓ ISO created: hello-os.iso"
elif command -v i686-elf-grub-mkrescue &> /dev/null; then
    i686-elf-grub-mkrescue -o hello-os.iso isodir
    echo "✓ ISO created: hello-os.iso"
elif command -v grub2-mkrescue &> /dev/null; then
    grub2-mkrescue -o hello-os.iso isodir
    echo "✓ ISO created: hello-os.iso"
else
    echo "ERROR: grub-mkrescue not found!"
    echo "Tried: grub-mkrescue, i686-elf-grub-mkrescue, grub2-mkrescue"
    exit 1
fi

echo ""
echo "Run with:"
echo "  qemu-system-x86_64 -cdrom hello-os.iso -serial stdio -nographic"
