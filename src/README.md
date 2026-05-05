# AIOS Kernel

> The core of the AI-generated operating system.

## Subsystems

| Directory | Description |
|-----------|-------------|
| `src/kernel/` | Kernel core (scheduler, syscall, panic) |
| `src/arch/x86_64/` | x86_64 architecture-specific code |
| `src/drivers/` | Hardware drivers |
| `src/fs/` | Filesystem implementations |
| `src/mm/` | Memory management |
| `src/net/` | Networking stack |
| `src/ipc/` | Inter-process communication |
| `src/lib/` | Core libraries |

## Getting Started

```bash
# Build
cargo build --release --features x86_64

# Run in QEMU
make run

# Run tests
cargo test
```

## AI Generation

All kernel code is AI-generated. See [CHANGELOG.md](../docs/CHANGELOG.md) for commit conventions.
