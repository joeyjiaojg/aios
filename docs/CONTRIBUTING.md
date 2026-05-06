# Contributing to AIOS

> All changes MUST go through Pull Requests. Direct pushes to `main`/`master` are not allowed.

## Workflow

1. **Create a branch** from `master`
2. **Make changes** following AI commit conventions
3. **Commit** with proper AI metadata (Model/Tool/Prompt)
4. **Push** your branch to GitHub
5. **Open a PR** targeting `master`
6. **AI reviews** your PR automatically
7. **Auto-merge** if AI approves

## Commit Convention

Every commit MUST include AI generation metadata:

```
<type>(<scope>): <description>

Model: MiniMax M2.5 Free
Tool: opencode
Prompt: <brief-prompt-summary>
```

### Types
- `feat` - New feature
- `fix` - Bug fix
- `test` - Tests
- `chore` - Maintenance
- `docs` - Documentation
- `refactor` - Code restructuring
- `perf` - Performance improvement

### Example

```
feat(mm): implement virtual memory manager with 4-level paging

Model: MiniMax M2.5 Free
Tool: opencode
Prompt: Create PML4-based virtual memory manager with page table allocation
```

## PR Requirements

- [ ] Commit messages follow AIOS convention
- [ ] Code includes tests
- [ ] No unsafe blocks without justification comments
- [ ] Compatible with x86_64-unknown-none target
- [ ] Code passes formatting check (`make fmt`)
- [ ] Code passes clippy check (`make clippy`)

## AI Auto-Merge

PRs are automatically reviewed and merged by the AI workflow:
- **Model**: MiniMax M2.5 Free
- **Tool**: opencode
- **Checks**: Memory safety, architecture correctness, style consistency

If AI rejects your PR, check the review comment and fix the issues.

---

**No direct pushes to master. All changes require PR review.**
