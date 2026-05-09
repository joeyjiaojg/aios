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

.PHONY: all build run clean test test-qemu test-unit test-integration fmt check clippy

all: build

build: $(KERNEL)

$(KERNEL): src/**/*.rs Cargo.toml
	mkdir -p $(BUILD)
	$(RUSTUP) run nightly $(CARGO) build --release --target $(TARGET)
	cp target/$(TARGET)/release/libaios_kernel.a $(KERNEL)

run: build
	$(QEMU) \
		-cdrom $(ISO) \
		-boot d \
		-serial stdio \
		-no-reboot \
		-m 256M

run-debug: build
	$(QEMU) \
		-cdrom $(ISO) \
		-boot d \
		-serial stdio \
		-no-reboot \
		-m 256M \
		-d int,cpu_reset \
		-D $(BUILD)/qemu.log

iso: build
	mkdir -p $(BUILD)/iso/boot/grub
	cp $(KERNEL) $(BUILD)/iso/boot/aios.kernel
	cp $(PROJECT_ROOT)/iso/boot/grub/grub.cfg $(BUILD)/iso/boot/grub/
	grub-mkrescue -o $(ISO) $(BUILD)/iso/

test: test-unit

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

