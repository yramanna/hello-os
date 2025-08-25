kernel := build/hello-os
prebuilt_iso := grub-prebuilt.iso
#prebuilt_iso :=

ifneq ($(prebuilt_iso),)
iso := $(prebuilt_iso)
else
iso := build/grub.iso
endif

grub_cfg := boot/grub.cfg

.PHONY: all
all: $(kernel)

.PHONY: clean
clean:
	rm -r build
	cargo clean

.PHONY: run
run: $(iso)
	qemu-system-x86_64 -vga std -s -serial file:serial.log \
		-boot d \
		-cdrom $(iso) \
		-drive file=fat:rw:$(PWD)/build,format=raw,media=disk

.PHONY: run-nox
run-nox: $(iso)
	qemu-system-x86_64 -nographic -s \
		-boot d \
		-cdrom $(iso) \
		-drive file=fat:rw:$(PWD)/build,format=raw,media=disk

.PHONY: iso
iso: $(iso)

ifeq ($(prebuilt_iso),)
$(iso): $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue --compress=xz -o $(iso) build/isofiles
	@rm -r build/isofiles
endif

.PHONY: kernel
kernel: $(kernel)

.PHONY: $(kernel)
$(kernel):
	cargo build --artifact-dir=$(PWD)/build

