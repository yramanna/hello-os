kernel := build/hello-os
prebuilt_iso := grub-prebuilt.iso
#prebuilt_iso :=

iso ?= build/hello-os.iso

ifneq ($(prebuilt_iso),)
stub_iso ?= $(prebuilt_iso)
else
stub_iso ?= build/grub.iso
endif

grub_stub_cfg := boot/grub.stub.cfg
grub_cfg := boot/grub.cfg

.PHONY: all
all: $(kernel)

.PHONY: clean
clean:
	rm -r build
	cargo clean

.PHONY: run
run: $(stub_iso) $(kernel)
	ISO=$(iso) STUB_ISO=$(stub_iso) ./qemu.sh

.PHONY: run-nox
run-nox: $(stub_iso) $(kernel)
	ISO=$(iso) STUB_ISO=$(stub_iso) ./qemu.sh -nographic

.PHONY: iso
iso: $(iso)

$(iso): $(grub_cfg) $(kernel)
	@mkdir -p build/isofiles/boot/grub
	cp $(grub_cfg) build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	grub-mkrescue --compress=xz -o $(iso) build/isofiles
	@rm -r build/isofiles

.PHONY: stub-iso
stub-iso: $(stub_iso)

ifeq ($(prebuilt_iso),)
$(stub_iso): $(grub_stub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(grub_stub_cfg) build/isofiles/boot/grub
	grub-mkrescue --compress=xz -o $(iso) build/isofiles
	@rm -r build/isofiles
endif

.PHONY: kernel
kernel: $(kernel)

.PHONY: $(kernel)
$(kernel):
	cargo build --artifact-dir=$(PWD)/build

