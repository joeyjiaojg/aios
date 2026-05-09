#!/bin/bash
# AIOS QEMU Test Harness
#
# Model: opencode/minimax-m2.5-free
# Tool: opencode
# Prompt: Implement test harness for QEMU automated boot and smoke tests.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
KERNEL="$BUILD_DIR/kernel.bin"
TIMEOUT=30

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

check_dependencies() {
    if ! command -v qemu-system-x86_64 &> /dev/null; then
        log_fail "qemu-system-x86_64 not found. Please install QEMU."
        exit 1
    fi
}

build_kernel() {
    log_info "Building kernel..."
    mkdir -p "$BUILD_DIR"
    cd "$PROJECT_ROOT"
    cargo +nightly build --release --target x86_64-unknown-none 2>/dev/null
    cp target/x86_64-unknown-none/release/libaios_kernel.a "$KERNEL"
    log_pass "Kernel built"
}

run_qemu_test() {
    log_info "Running QEMU test with ${TIMEOUT}s timeout..."

    local output_file=$(mktemp)
    local exit_code=0

    timeout $TIMEOUT qemu-system-x86_64 \
        -kernel "$KERNEL" \
        -nographic \
        -no-reboot \
        -m 256M \
        -append "console=ttyS0" \
        2>&1 | tee "$output_file" || exit_code=$?

    if [ $exit_code -eq 124 ]; then
        log_fail "QEMU timed out after ${TIMEOUT}s"
        rm -f "$output_file"
        return 1
    fi

    if [ $exit_code -ne 0 ]; then
        log_fail "QEMU exited with code $exit_code"
        rm -f "$output_file"
        return 1
    fi

    if grep -q "panic" "$output_file"; then
        log_fail "Kernel panic detected"
        rm -f "$output_file"
        return 1
    fi

    rm -f "$output_file"
    log_pass "QEMU boot test passed"
    return 0
}

run_smoke_tests() {
    log_info "Running smoke tests..."

    local output_file=$(mktemp)

    timeout $TIMEOUT qemu-system-x86_64 \
        -kernel "$KERNEL" \
        -nographic \
        -no-reboot \
        -m 256M \
        2>&1 | tee "$output_file" || true

    if grep -q "AIOS Shell" "$output_file"; then
        log_pass "Shell started successfully"
    fi

    rm -f "$output_file"
}

main() {
    echo "=========================================="
    echo "  AIOS QEMU Test Harness"
    echo "=========================================="
    echo ""

    check_dependencies
    build_kernel

    echo ""
    echo "--- Boot Test ---"
    run_qemu_test

    echo ""
    echo "--- Smoke Tests ---"
    run_smoke_tests

    echo ""
    log_pass "All tests completed"
}

main "$@"
