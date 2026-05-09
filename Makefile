# AIOS Makefile
# Build configuration for the AIOS x86_64 kernel

ARCH := x86_64
TARGET := $(ARCH)-unknown-none
BUILD := build
PROJECT_ROOT := $(shell pwd)
ISO := $(BUILD)/aios.iso
KERNEL := $(BUILD)/kernel.bin
QEMU := qemu-system-x86_64
RUSTUP := ${HOME}/.cargo/bin/rustup
CARGO := ${HOME}/.cargo/bin/cargo

.PHONY: all build clean test test-qemu test-unit test-integration fmt check clippy deps iso run run-debug

all: deps build

deps:
	@echo "Checking dependencies..."
	@which qemu-system-x86_64 > /dev/null 2>&1 || (echo "ERROR: qemu-system-x86_64 not found. Install with: sudo apt-get install qemu-system-x86" && exit 1)
	@which xorriso > /dev/null 2>&1 || (echo "ERROR: xorriso not found. Install with: sudo apt-get install xorriso" && exit 1)
	@which grub-mkrescue > /dev/null 2>&1 || (echo "ERROR: grub-mkrescue not found. Install with: sudo apt-get install grub-common" && exit 1)
	@which mformat > /dev/null 2>&1 || (echo "ERROR: mformat not found. Install with: sudo apt-get install mtools" && exit 1)
	@echo "All dependencies satisfied."

build: deps $(KERNEL)

$(KERNEL): src/**/*.rs src/**/**/*.S
	mkdir -p $(BUILD)
	$(RUSTUP) run nightly $(CARGO) build --release --target $(TARGET)

iso: deps build
	mkdir -p $(BUILD)/iso/boot/grub
	cp $(KERNEL) $(BUILD)/iso/boot/aios.kernel
	cp $(PROJECT_ROOT)/iso/boot/grub/grub.cfg $(BUILD)/iso/boot/grub/
	GRUB_PLATFORM=i386-pc grub-mkrescue -o $(ISO) $(BUILD)/iso/ 2>&1 || (echo "ERROR: grub-mkrescue failed. Try installing grub-pc-bin: sudo apt-get install grub-pc-bin" && exit 1)

run: deps iso
	$(QEMU) \
		-nographic \
		-cdrom $(ISO) \
		-boot d \
		-serial file:$(BUILD)/serial.log \
		-no-reboot \
		-m 256M

run-uefi: deps iso
	$(QEMU) \
		-nographic \
		-drive if=pflash,format=raw,file=/usr/share/ovmf/OVMF.fd \
		-cdrom $(ISO) \
		-boot d \
		-serial file:$(BUILD)/serial.log \
		-no-reboot \
		-m 256M

run-debug: deps iso
	$(QEMU) \
		-nographic \
		-cdrom $(ISO) \
		-boot d \
		-serial file:$(BUILD)/serial.log \
		-no-reboot \
		-m 256M \
		-d int,cpu_reset \
		-D $(BUILD)/qemu.log

test-unit:
	cd src/lib/boot_info && $(RUSTUP) run nightly $(CARGO) test --lib

test-integration:
	$(RUSTUP) run nightly $(CARGO) test --test integration --target $(TARGET) -Zbuild-std=core,alloc

test-qemu:
	@echo "Running QEMU automated tests..."
	@chmod +x test/qemu/run_tests.sh
	@test/qemu/run_tests.sh

# Code quality checks
fmt:
	$(RUSTUP) run nightly $(CARGO) fmt --all -- --check

clippy:
	$(RUSTUP) run nightly $(CARGO) clippy --lib -- -D warnings

ai-review:
	@echo "Running AI code review using opencode..."
	@mkdir -p /tmp
	@git diff origin/master...HEAD > /tmp/pr_diff.txt 2>/dev/null || git diff HEAD > /tmp/pr_diff.txt
	@timeout 240 opencode run -m opencode/minimax-m2.5-free \
		"You are a code reviewer for AIOS, an AI-generated x86_64 OS written in Rust.\n\nReview the following PR diff:\n- Memory safety (unsafe blocks need justification)\n- x86_64 architecture correctness\n- Consistency with Rust no_std patterns\n- No hardcoded secrets\n- Proper error handling\n- Test coverage\n- Commit message follows AIOS convention (Model/Tool fields)\n\nDiff:\n$$(cat /tmp/pr_diff.txt)\n\nOutput exactly 'APPROVED' or 'REJECTED' with brief reasoning." || (echo "Review failed or timed out"; exit 1)

check: fmt clippy test

clean:
	rm -rf $(BUILD)
	rm -rf target
	cargo clean

