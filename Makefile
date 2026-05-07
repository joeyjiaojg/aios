# AIOS Makefile
# Build configuration for the AIOS x86_64 kernel

ARCH := x86_64
TARGET := $(ARCH)-unknown-none
BUILD := build
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
	grub-mkrescue -o $(ISO) $(BUILD)/iso/

test: test-unit

test-unit:
	cd src/lib/boot_info && $(RUSTUP) run nightly $(CARGO) test --lib

test-integration:
	$(RUSTUP) run nightly $(CARGO) test --test integration --target $(TARGET) -Zbuild-std=core,alloc

# Code quality checks
fmt:
	$(RUSTUP) run nightly $(CARGO) fmt --all -- --check

clippy:
	$(RUSTUP) run nightly $(CARGO) clippy --lib -- -D warnings

check: fmt clippy test

clean:
	rm -rf $(BUILD)
	rm -rf target
	cargo clean
