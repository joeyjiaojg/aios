# AIOS - Claude Code Agent Guidelines

## Quick Reference
- Project: AIOS x86_64 kernel in Rust (no_std)
- Key rules: No Vec/String, # Safety comments required, minimum 10 tests
- Workflow: Use `skill:workflow-enforcer pre-push` before push, `post-merge` after merge

## Critical Rules

### 1. Code Style
- All kernel code must be `no_std` compatible
- Never use `Vec`, `String`, or `alloc` types
- Use fixed-size arrays: `[T; N]` instead of `Vec<T>`
- Files must end with a newline character

### 2. Safety Comments
```rust
// # Safety
// <explain why this is safe>
```
Required for all `unsafe` blocks.

### 3. Testing
- Minimum 10 tests per module
- Test naming: `test_<module>_<function>_<scenario>`

### 4. Commit Format
```
<type>(<scope>): <subject>

- <description>
- Model: opencode/minimax-m2.5-free
- Tool: opencode
- Prompt: <actual prompt used>
```

## Workflow Enforcement (IMPORTANT!)

**Use skill:workflow-enforcer BEFORE any git push:**

```bash
# Before any push (mandatory check)
skill:workflow-enforcer pre-push

# After PR merges
skill:workflow-enforcer post-merge <pr_number>

# Cleanup merged branches
skill:workflow-enforcer cleanup-branches
```

**Rules:**
- ❌ FORBIDDEN: Push directly to `master` - always use PR workflow
- ❌ FORBIDDEN: Run `gh pr merge` manually - let auto-merge handle it
- ✅ MUST: Close issue when PR merges (check "Closes #N" in PR body)
- ✅ MUST: Delete merged branches (local and remote)
- ✅ MUST: Use PR workflow for all changes

## PR Workflow
1. `git fetch origin master && git checkout master && git pull`
2. `git checkout -b feat/<feature-name>`
3. Run `make check` - MUST pass before push
4. `git push origin feat/<feature-name>` - NEVER to master
5. `gh pr create --title "..." --body "..."`
6. Check AI Review Result in PR comments (not CI status)
7. If REJECTED: Fix issues, push again
8. If APPROVED: Wait for auto-merge (don't run `gh pr merge`)

## Common Mistakes
❌ Using `Vec` in `no_std` code
❌ Forgetting newline at end of file
❌ Missing `# Safety` comments on `unsafe` blocks
❌ Pushing directly to master
❌ Force pushing without explicit user approval
❌ Running `gh pr merge` manually

## Push / Force-Push Policy (ENFORCED BY HOOK)

These rules are enforced by a PreToolUse hook in `~/.claude/settings.json`
and apply **even in `dangerously-skip-permissions` mode**:

| Action | Result |
|--------|--------|
| `git push … master` or `… :master` | **Hard blocked** (`exit 1`) — no override |
| `git push --force` / `-f` (any branch) | **Blocked + user approval required** (`exit 2`) |
| `gh pr merge` | **Hard blocked** (`exit 1`) — let auto-merge handle it |

If you need to force-push to a feature branch, the hook will pause and
ask the user explicitly — do NOT attempt to work around it.

## Self-Evolution
- Self-evolve runs every 30 minutes on schedule
- Skips issues with labels, comments, or existing PRs
- Creates `feat/auto-issue-N` branches
- Requires AI review before merge

## Skill Location
`.github/skills/workflow-enforcer/`
