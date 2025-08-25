kernel := build/hello-os
iso := build/hello.iso

grub_cfg := boot/grub.cfg

.PHONY: all
all: $(kernel)

.PHONY: clean
clean:
	rm -r build
	cargo clean

.PHONY: run
run: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -vga std -s -serial file:serial.log

.PHONY: run-nox
run-nox: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -nographic -s

.PHONY: iso
iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

.PHONY: kernel
kernel: $(kernel)

.PHONY: $(kernel)
$(kernel):
	cargo build --artifact-dir=$(PWD)/build

