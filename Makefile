kernel := build/hello-os
# prebuilt_iso := grub-prebuilt.iso
# #prebuilt_iso :=

# iso ?= build/hello-os.iso

# ifneq ($(prebuilt_iso),)
# stub_iso ?= $(prebuilt_iso)
# else
# stub_iso ?= build/grub.iso
# endif

# grub_stub_cfg := boot/grub.stub.cfg
iso := build/hello-os.iso
grub_cfg := boot/grub.cfg

.PHONY: all
all: $(kernel)

.PHONY: clean
clean:
	rm -r build
	rm -f $(iso)
	cargo clean

.PHONY: run
# run: $(stub_iso) $(kernel)
# 	ISO=$(iso) STUB_ISO=$(stub_iso) ./qemu.sh
run: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -nographic

.PHONY: run-nox
# run-nox: $(stub_iso) $(kernel)
# 	ISO=$(iso) STUB_ISO=$(stub_iso) ./qemu.sh -nographic
run-nox: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -nographic

.PHONY: run-gdb
run-gdb: $(stub_iso) $(kernel)
# 	ISO=$(iso) STUB_ISO=$(stub_iso) ./qemu.sh -S
	qemu-system-x86_64 -cdrom $(iso) -nographic

.PHONY: run-nox-gdb
run-nox-gdb: $(stub_iso) $(kernel)
# 	ISO=$(iso) STUB_ISO=$(stub_iso) ./qemu.sh -nographic -S
	qemu-system-x86_64 -cdrom $(iso) -nographic

.PHONY: iso
iso: $(iso)

$(iso): $(kernel)
	@echo "Creating bootable ISO..."
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/hello-os
	@echo 'set timeout=0' > build/isofiles/boot/grub/grub.cfg
	@echo 'set default=0' >> build/isofiles/boot/grub/grub.cfg
	@echo '' >> build/isofiles/boot/grub/grub.cfg
	@echo 'menuentry "Hello OS" {' >> build/isofiles/boot/grub/grub.cfg
	@echo '    multiboot2 /boot/hello-os' >> build/isofiles/boot/grub/grub.cfg
	@echo '    boot' >> build/isofiles/boot/grub/grub.cfg
	@echo '}' >> build/isofiles/boot/grub/grub.cfg
	@if command -v i686-elf-grub-mkrescue >/dev/null 2>&1; then \
		i686-elf-grub-mkrescue -o $(iso) build/isofiles 2>&1 | grep -v "xorriso" || true; \
	elif command -v grub-mkrescue >/dev/null 2>&1; then \
		grub-mkrescue -o $(iso) build/isofiles 2>&1 | grep -v "xorriso" || true; \
	elif command -v grub2-mkrescue >/dev/null 2>&1; then \
		grub2-mkrescue -o $(iso) build/isofiles 2>&1 | grep -v "xorriso" || true; \
	else \
		echo "ERROR: No grub-mkrescue found!"; \
		exit 1; \
	fi
	@rm -rf build/isofiles
	@echo "✓ ISO created: $(iso)"

.PHONY: stub-iso
stub-iso: $(stub_iso)

ifeq ($(prebuilt_iso),)
$(stub_iso): $(grub_stub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(grub_stub_cfg) build/isofiles/boot/grub
	grub-mkrescue --compress=xz -o $(iso) -d $(GRUB_X86_MODULES) build/isofiles
	@rm -r build/isofiles
endif

.PHONY: kernel
kernel: $(kernel)

.PHONY: $(kernel)
$(kernel):
	cargo build --artifact-dir=$(PWD)/build

.PHONY: gdb
gdb:
	gdb -iex "set auto-load local-gdbinit off" -iex "source ./.gdbinit"
