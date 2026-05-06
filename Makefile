# AIOS Makefile

ARCH := x86_64
TARGET := $(ARCH)-unknown-none
BUILD := build
ISO := $(BUILD)/aios.iso
KERNEL := $(BUILD)/kernel.bin
QEMU := qemu-system-x86_64

.PHONY: all build run clean test test-qemu test-unit test-integration fmt check clippy

all: build

build: $(KERNEL)

$(KERNEL): src/**/*.rs Cargo.toml
	cargo build --release --target $(TARGET) --features $(ARCH)
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

test: test-unit test-integration test-qemu

test-unit:
	cargo test --lib

test-integration:
	cargo test --test integration

test-qemu: build
	$(QEMU) \
		-cdrom $(ISO) \
		-serial file:$(BUILD)/qemu-output.txt \
		-no-reboot \
		-m 128M \
		-display none

# Code quality checks
fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt clippy test

clean:
	rm -rf $(BUILD)
	rm -rf target
	cargo clean
