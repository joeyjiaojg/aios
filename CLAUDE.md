# CLAUDE.md for AIOS Project

## Project Context
- **Project**: AIOS - x86_64 OS kernel in Rust
- **Target**: `no_std` environment (bare metal)
- **Key dependencies**: `spin`, `x86_64`

## Rules for Claude Code / OpenCode

### Mandatory File Header
Every `.rs` file MUST start with:
```rust
// AIOS <Module Name>
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: <description>
```

### no_std Constraints
- ❌ **NEVER** use: `Vec`, `String`, `Box`, `alloc`
- ✅ **USE**: Fixed arrays `[T; N]`, `spin::Mutex`, `core::` types
- ✅ **USE**: `[0u8; SIZE]` for data buffers

### Safety Rules
1. Avoid `unsafe` when possible
2. If `unsafe` is needed:
   - Add `/// # Safety` doc comment
   - Explain why it's safe
3. Prefer fixed arrays over raw pointers

### Commit Format
```
type(scope): subject

- description
- Model: opencode/minimax-m2.5-free
- Tool: opencode
- Prompt: <prompt>
```

### Testing
- Min 10 tests per module
- No `Vec` in tests
- Test file: `src/kernel/<module>.rs` (within `#[cfg(test)]`)

### Common Errors (Learned the Hard Way)
1. **PR#6 REJECTED 5 times** because:
   - Used `Vec` in `no_std` code
   - `unsafe` block without `# Safety` comment
   - Forgot newline at EOF
   - Test assertion wrong (`node_count == 0` but root exists)
   
2. **PR#5 didn't exist** - should have created from Issue#5

### Workflow
1. Check Issue#N exists
2. Create branch `feat/<name>`
3. Code + test
4. Commit with proper format
5. Push + create PR#N (match Issue number)
6. Wait for AI Review
7. Fix REJECTED issues → goto 4

### Quick Links
- Issues: https://github.com/joeyjiaojg/aios/issues
- PRs: https://github.com/joeyjiaojg/aios/pulls
