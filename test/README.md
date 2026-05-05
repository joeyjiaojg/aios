# AIOS Tests

> Comprehensive testing for the AI-generated operating system.

## Test Structure

| Directory | Purpose |
|-----------|---------|
| `unit/` | Per-module unit tests |
| `integration/` | Cross-subsystem integration tests |
| `qemu/` | QEMU-based automated boot and syscall tests |

## Running Tests

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
make test-integration
```

### QEMU Tests
```bash
make test-qemu
```

## Test Requirements

- **Unit tests**: Every module must have corresponding unit tests
- **Integration tests**: New syscalls require integration test coverage
- **QEMU tests**: Boot success and basic syscall tests run on every PR
- **Coverage target**: 80%+ code coverage

## AI-Generated Tests

All tests are AI-generated alongside the code they test. The AI research workflow (`.github/workflows/ai-research.yml`) also generates new test cases based on feature research.
