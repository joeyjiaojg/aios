# AIOS Debug Flag Documentation

## Overview

AIOS includes a dynamic debug flag mechanism to control verbose kernel output. By default, debug output is **disabled** to provide clean command output matching standard Linux/Debian behavior. Debug mode can be enabled at runtime for troubleshooting and development.

## Debug Flag Control

### Runtime Control (Kernel Code)

Enable debug output from kernel code:

```rust
crate::debug::enable_debug();
```

Disable debug output:

```rust
crate::debug::disable_debug();
```

Toggle debug state:

```rust
crate::debug::toggle_debug();
```

Check current debug state:

```rust
if crate::debug::is_debug_enabled() {
    // Debug is enabled
}
```

### Conditional Debug Output

All debug output in the kernel is wrapped with conditional checks:

```rust
if crate::debug::is_debug_enabled() {
    crate::serial::write_str("[module] debug message\r\n");
}
```

## Modules with Debug Output

The following kernel modules have debug output controlled by the debug flag:

### 1. ELF Loader (`src/kernel/elf.rs`)
- **map_user_segment()** - Page table mapping operations
- **load_segments()** - ELF segment loading
- **setup_user_context()** - User context initialization
- **start_user_program()** - Ring-3 transition setup

Debug output includes:
- Virtual address ranges being mapped
- Page table entry modifications
- ELF segment loading progress
- iretq frame construction details

### 2. Ramdisk Filesystem (`src/kernel/ramdisk.rs`)
- **init_from_modules()** - Ramdisk initialization
- **lookup_file()** - File lookup operations

Debug output includes:
- Module registration
- File lookup results
- Data source (embedded vs module)

### 3. Shell Execution (`src/kernel/shell/builtins.rs`)
- **exec_cmd()** - Program execution

Debug output includes:
- Program loading progress
- GDT selector information
- Frame allocator setup
- ELF setup status
- Entry point and stack addresses

## Usage Examples

### Example 1: Enable Debug for Troubleshooting

In kernel initialization or a syscall handler:

```rust
// Enable debug output
crate::debug::enable_debug();

// Run busybox command
crate::shell::builtins::exec_cmd("/bin/busybox", &["/bin/busybox", "ls"]);

// Disable debug output
crate::debug::disable_debug();
```

### Example 2: Debug Specific Operations

```rust
// Enable debug only for ELF loading
crate::debug::enable_debug();
let context = crate::elf::setup_user_context(elf_data, allocator, phys_base, args)?;
crate::debug::disable_debug();
```

### Example 3: Conditional Debug Based on Environment

```rust
// Enable debug if a specific process is running
if process_name == "debug_target" {
    crate::debug::enable_debug();
}
```

## Debug Output Format

All debug messages follow a consistent format:

```
[module] operation: details
```

Examples:
```
[elf] map_user_segment: vaddr=0x400000 memsz=0x1000
[elf] load_segments: loading to vaddr=0x400000 filesz=0x800 memsz=0x1000
[ramdisk] lookup: /bin/busybox
[ramdisk] found: /bin/busybox
```

## Performance Considerations

- **Debug disabled (default)**: Minimal overhead - only a single atomic boolean read per debug point
- **Debug enabled**: Full debug output to serial console (may slow down execution)
- **Atomic operations**: Uses `AtomicBool` with relaxed ordering for thread-safe access
- **No compile-time overhead**: Debug checks are runtime decisions, not compile-time flags

## Implementation Details

### Debug Module (`src/kernel/debug.rs`)

The debug module provides:

- **DEBUG_ENABLED**: Static `AtomicBool` initialized to `false`
- **Atomic operations**: Uses `Ordering::Relaxed` for performance
- **Unit tests**: 4 tests verify enable/disable/toggle functionality
- **Helper macros**: `debug_println!` and `debug_write!` for future use

### Thread Safety

The debug flag is thread-safe:
- Uses `AtomicBool` for atomic access
- No locks required
- Safe for concurrent reads/writes

## Future Enhancements

Potential improvements to the debug system:

1. **Debug levels** - Separate debug, info, warn, error levels
2. **Module-specific flags** - Enable/disable debug per module
3. **Kernel command-line parameter** - `debug=1` boot parameter
4. **Serial console commands** - Interactive debug toggle via serial
5. **Performance profiling** - Measure overhead of debug output

## Testing

Run debug flag tests:

```bash
cargo test --lib debug
```

Tests verify:
- Debug disabled by default
- Enable/disable functionality
- Toggle functionality
- State persistence

## See Also

- [FEATURES.md](FEATURES.md) - Feature overview
- [CONTRIBUTING.md](CONTRIBUTING.md) - Development guidelines
- [src/kernel/debug.rs](../src/kernel/debug.rs) - Debug module source
