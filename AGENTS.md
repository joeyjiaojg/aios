# AIOS AI Agent Guidelines

## Project Overview
AIOS is an x86_64 operating system kernel written in Rust, targeting `no_std` environment.

## Critical Rules for AI Agents

### 1. Code Style & Conventions
- **All kernel code must be `no_std` compatible**
- **Never use `Vec`, `String`, or `alloc` types** unless explicitly feature-gated
- Use fixed-size arrays: `[T; N]` instead of `Vec<T>`
- Use `spin::Mutex` for thread safety (already in dependencies)
- Files must end with a newline character

### 2. Comment Headers (Required in every file)
```rust
// AIOS <Module Name>
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: <what you were asked to do>
```

### 3. Commit Message Format (Required)
```
<type>(<scope>): <subject>

- <description>
- Model: opencode/minimax-m2.5-free
- Tool: opencode
- Prompt: <actual prompt used>
```

### 4. Safety Critical (IMPORTANT!)
- **Avoid `unsafe` blocks when possible**
- If `unsafe` is necessary:
  - Add `# Safety` doc comment explaining why it's safe
  - Justify every raw pointer dereference
- **Preferred**: Use fixed-size arrays over raw pointers (`Option<*mut u8>` → `[u8; N]`)

### 5. Testing Requirements
- Minimum 10 tests per module
- Test naming: `test_<module>_<function>_<scenario>`
- All tests must compile in `no_std` environment
- Never use `Vec` in test code

### 6. PR Workflow
1. **Create branch**: `git checkout -b feat/<feature-name>`
2. **Commit changes** with proper format (see above)
3. **Push**: `git push origin feat/<feature-name>`
4. **Create PR**: `gh pr create --title "feat(scope): description" --body "..."`
5. **Wait for CI + AI Review** (AI Auto-Merge workflow)
6. **Only merge if AI Review PASSED** - NEVER merge when AI Review REJECTED
7. **If REJECTED**: Fix ALL issues, commit, push again
8. **Never force-push** unless explicitly asked
9. **Never manually merge** when AI Review shows REJECTED

### 7. Common Mistakes to Avoid
❌ Using `Vec` in `no_std` code
❌ Forgetting newline at end of file
❌ Missing `# Safety` comments on `unsafe` blocks
❌ assertion errors in tests (e.g., expecting `node_count == 0` when root node exists)
❌ Raw pointers when fixed arrays work (`Option<*mut u8>` → `[u8; MAX_DATA_SIZE]`)
❌ Creating PR#5 when Issue#5 exists (should match numbers)

### 8. Git Branching Strategy
- `master`: stable, merged code only
- `feat/<feature>`: feature branches (PR from here)
- PR numbers should match Issue numbers when applicable

### 9. AI Review Criteria (What AI checks)
✅ Memory safety (no unjustified `unsafe`)
✅ x86_64 architecture correctness
✅ `no_std` compatibility (no `Vec`, `String`, etc.)
✅ No hardcoded secrets
✅ Proper error handling
✅ Test coverage (minimum 10 tests)
✅ Commit message has Model/Tool/Prompt fields

### 10. Quick Reference
| Issue | PR | Feature |
|-------|----|---------|
| #1 | #1 | Virtual Memory Manager |
| #2 | #2 | Task/Process Manager |
| #3 | #3 | Enhanced Interrupts |
| #5 | #7 | Device Driver Framework |
| #6 | #6 | VFS Framework v4 |

## Emergency Debugging
If PR gets REJECTED repeatedly:
1. Read the REJECTED reason carefully
2. Check if `Vec` is used → replace with fixed array
3. Check if `unsafe` blocks have `# Safety` comments
4. Check if file ends with newline
5. Check test assertions match implementation
6. Commit fixes with reference to REJECTED reason
