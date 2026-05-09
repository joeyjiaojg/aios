# AIOS Workflow Enforcer

## Overview
This skill enforces AIOS workflow rules to ensure proper development practices.

## When to Use
- Before running `git push origin <branch>` - call `skill:workflow-enforcer pre-push`
- After PR is merged - call `skill:workflow-enforcer post-merge <pr_number>`
- When cleaning up merged branches - call `skill:workflow-enforcer cleanup-branches`
- When auditing master for direct pushes - call `skill:workflow-enforcer audit`

## Commands

### pre-push
```bash
skill:workflow-enforcer pre-push
```
Checks if you're about to push to master. **FORBIDDEN** - must use PR workflow.

### post-merge
```bash
skill:workflow-enforcer post-merge <pr_number>
```
After PR merge:
1. Extracts issue number from PR body (e.g., "Closes #42")
2. Closes the associated issue
3. Deletes the merged branch (local and remote)

### cleanup-branches
```bash
skill:workflow-enforcer cleanup-branches
```
Cleans up all branches from merged PRs.

### audit
```bash
skill:workflow-enforcer audit
```
Audits master branch for any direct pushes (violations).

## Rules Enforced
- ❌ NEVER push directly to `master` - always use PR workflow
- ❌ NEVER run `gh pr merge` manually - let auto-merge handle it
- ✅ MUST close issue when PR merges (extracts "Closes #N" from PR body)
- ✅ MUST delete merged branches
- ✅ MUST use PR workflow for all changes

## Files
- `.github/skills/workflow-enforcer/enforce-workflow.sh` - the enforcement script
- `scripts/enforce-workflow.sh` - copy of the script in scripts/ directory

## Skill Trigger Keywords
Use when agent mentions:
- "enforce workflow"
- "check before push"
- "cleanup branches"
- "post-merge"
- "pre-push check"
- "audit workflow"
- Or before any `git push` operation