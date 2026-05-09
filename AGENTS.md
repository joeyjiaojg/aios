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
1. **Fetch and sync**: `git fetch origin master && git checkout master && git pull`
2. **Create branch**: `git checkout -b feat/<feature-name>`
3. **Commit changes** with proper format (see above)
4. **Run checks**: `make check` (runs `fmt`, `clippy`, and `test-unit`) - MUST pass before pushing
5. **Push to branch**: `git push origin feat/<feature-name>` - NEVER push directly to master
6. **Create PR**: `gh pr create --title "feat(scope): description" --body "..."`
7. **Check AI Review Result**: After PR is created/updated:
   - Run `gh pr view <number> --json comments` to get comments
   - Look for a comment from `github-actions` containing "AI Review Result"
   - Check the **Status** field in the comment body for REJECTED/APPROVED
   - **IMPORTANT**: The CI check status (`gh pr checks`) may show PASS even when AI Review says REJECTED
   - Always check the comment content for the actual result
8. **If REJECTED**: Fix ALL issues, commit, push again, then re-check AI Review Result
9. **If APPROVED**: Wait for GitHub auto-merge workflow - DO NOT run `gh pr merge` manually
10. **NEVER merge a REJECTED PR** - continue fixing until APPROVED
11. **Never force-push** unless explicitly asked
12. **Never push directly to master** - always use PR workflow
13. **Never run `gh pr merge` manually** - let the GitHub workflow auto-merge approved PRs

### 7. Common Mistakes to Avoid
❌ Using `Vec` in `no_std` code
❌ Forgetting newline at end of file
❌ Missing `# Safety` comments on `unsafe` blocks
❌ assertion errors in tests (e.g., expecting `node_count == 0` when root node exists)
❌ Raw pointers when fixed arrays work (`Option<*mut u8>` → `[u8; MAX_DATA_SIZE]`)
❌ Creating PR#5 when Issue#5 exists (should match numbers)
❌ Pushing directly to master - always use PR workflow
❌ Running `gh pr merge` manually - let auto-merge workflow handle it

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
| #40 | #53 | Timer Interrupt and Scheduler |
| #43 | #48 | GRUB/Multiboot Configuration |

## Emergency Debugging
If PR gets REJECTED repeatedly:
1. Read the REJECTED reason carefully
2. Check if `Vec` is used → replace with fixed array
3. Check if `unsafe` blocks have `# Safety` comments
4. Check if file ends with newline
5. Check test assertions match implementation
6. Commit fixes with reference to REJECTED reason
