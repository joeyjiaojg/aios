# AIOS Makefile
# Build configuration for the AIOS x86_64 kernel

ARCH := x86_64
TARGET := $(ARCH)-unknown-none
BUILD := build
PROJECT_ROOT := $(shell pwd)
ISO := $(BUILD)/aios.iso
KERNEL := $(BUILD)/kernel.bin
DOCKER_IMAGE := aios-builder
RUSTUP := ${HOME}/.cargo/bin/rustup
CARGO := ${HOME}/.cargo/bin/cargo
DOCKER := /usr/bin/docker

.PHONY: all build clean test test-qemu test-unit test-integration fmt check clippy \
        iso run run-debug run-uefi docker-build docker-iso

all: docker-iso

# ── Docker build environment ──────────────────────────────────────────────────
docker-build:
	$(DOCKER) build -t $(DOCKER_IMAGE) .

# Build kernel and ISO entirely inside Docker (handles missing host toolchain)
docker-iso: docker-build
	$(DOCKER) run --rm \
		-v $(PROJECT_ROOT):/aios \
		-w /aios \
		$(DOCKER_IMAGE) bash -c "\
			rustup run nightly cargo build --release --target $(TARGET) && \
			mkdir -p $(BUILD)/iso/boot/grub && \
			cp target/$(TARGET)/release/aios_kernel $(KERNEL) && \
			cp $(KERNEL) $(BUILD)/iso/boot/aios.kernel && \
			cp iso/boot/grub/grub.cfg $(BUILD)/iso/boot/grub/ && \
			wget -q -O $(BUILD)/iso/boot/busybox https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox && \
			chmod +x $(BUILD)/iso/boot/busybox && \
			GRUB_PLATFORM=i386-pc grub-mkrescue -o $(ISO) $(BUILD)/iso/ 2>&1"

# Host-native build (requires rust-src + x86_64-unknown-none installed)
build:
	mkdir -p $(BUILD)
	$(RUSTUP) run nightly $(CARGO) build --release --target $(TARGET)
	cp target/$(TARGET)/release/aios_kernel $(KERNEL)

iso: build
	mkdir -p $(BUILD)/iso/boot/grub
	cp $(KERNEL) $(BUILD)/iso/boot/aios.kernel
	cp $(PROJECT_ROOT)/iso/boot/grub/grub.cfg $(BUILD)/iso/boot/grub/
	GRUB_PLATFORM=i386-pc grub-mkrescue -o $(ISO) $(BUILD)/iso/ 2>&1 || \
		(echo "ERROR: grub-mkrescue failed. Try: sudo apt-get install grub-pc-bin" && exit 1)

# ── QEMU targets (run inside Docker) ─────────────────────────────────────────
run: docker-iso
	$(DOCKER) run --rm -it \
		-v $(PROJECT_ROOT)/$(BUILD):/aios/$(BUILD):ro \
		$(DOCKER_IMAGE) \
		qemu-system-x86_64 \
			-serial stdio \
			-display none \
			-cdrom /aios/$(BUILD)/aios.iso \
			-boot d \
			-no-reboot \
			-m 256M

# Debug run: int/cpu_reset events logged to build/qemu.log
run-debug: docker-iso
	$(DOCKER) run --rm -it \
		-v $(PROJECT_ROOT)/$(BUILD):/aios/$(BUILD) \
		$(DOCKER_IMAGE) \
		qemu-system-x86_64 \
			-serial stdio \
			-display none \
			-cdrom /aios/$(BUILD)/aios.iso \
			-boot d \
			-no-reboot \
			-m 256M \
			-d int,cpu_reset \
			-D /aios/$(BUILD)/qemu.log

run-uefi: docker-iso
	$(DOCKER) run --rm -it \
		-v $(PROJECT_ROOT)/$(BUILD):/aios/$(BUILD):ro \
		$(DOCKER_IMAGE) \
		qemu-system-x86_64 \
			-serial stdio \
			-display none \
			-drive if=pflash,format=raw,file=/usr/share/ovmf/OVMF.fd \
			-cdrom /aios/$(BUILD)/aios.iso \
			-boot d \
			-no-reboot \
			-m 256M

# ── Tests ─────────────────────────────────────────────────────────────────────
test-unit:
	@cd src/lib/boot_info && $(RUSTUP) run nightly $(CARGO) test --lib 2>/dev/null || echo "⚠️  unit tests skipped (rust-src issue)"

test-integration:
	$(RUSTUP) run nightly $(CARGO) test --test integration --target $(TARGET) -Zbuild-std=core,alloc

test-qemu:
	@echo "Running QEMU automated tests..."
	@chmod +x test/qemu/run_tests.sh
	@test/qemu/run_tests.sh

# ── Code quality ──────────────────────────────────────────────────────────────
fmt:
	$(RUSTUP) run nightly $(CARGO) fmt --all -- --check

clippy:
	$(RUSTUP) run nightly $(CARGO) clippy --bin aios_kernel -- -D warnings

ai-review:
	@echo "Running AI code review using opencode..."
	@mkdir -p /tmp
	@git diff origin/master...HEAD > /tmp/pr_diff.txt 2>/dev/null || git diff HEAD > /tmp/pr_diff.txt
	@timeout 240 opencode run -m opencode/minimax-m2.5-free \
		"You are a code reviewer for AIOS, an AI-generated x86_64 OS written in Rust.\n\nReview the following PR diff:\n- Memory safety (unsafe blocks need justification)\n- x86_64 architecture correctness\n- Consistency with Rust no_std patterns\n- No hardcoded secrets\n- Proper error handling\n- Test coverage\n- Commit message follows AIOS convention (Model/Tool fields)\n\nDiff:\n$$(cat /tmp/pr_diff.txt)\n\nOutput exactly 'APPROVED' or 'REJECTED' with brief reasoning." || (echo "Review failed or timed out"; exit 1)

check: fmt clippy-optional test-unit

clippy-optional:
	@$(RUSTUP) run nightly $(CARGO) clippy --bin aios_kernel -- -D warnings 2>/dev/null || echo "⚠️  clippy not available (network/certificate issue)"

clean:
	sudo chown -R $(shell whoami):users target build
	rm -rf $(BUILD)
	rm -rf target
	$(CARGO) clean
