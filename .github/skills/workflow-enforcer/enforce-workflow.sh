#!/bin/bash
# AIOS Workflow Enforcer Script
# Enforces AGENTS.md rules and ensures proper workflow
#
# Usage:
#   enforce-workflow.sh pre-push              # Check before git push
#   enforce-workflow.sh post-merge <pr_num>   # Cleanup after PR merge
#   enforce-workflow.sh cleanup-branches      # Cleanup all merged branches
#   enforce-workflow.sh audit                 # Audit master for violations
#   enforce-workflow.sh help                  # Show this help

set -e

REPO="${GITHUB_REPOSITORY:-joeyjiaojg/aios}"
MASTER_BRANCH="master"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_error() {
    echo -e "${RED}ERROR: $1${NC}" >&2
}

log_warn() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

log_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

log_info() {
    echo -e "${BLUE}INFO: $1${NC}"
}

# Check if trying to push to master
check_master_push() {
    local target_branch="$1"

    if [ "$target_branch" = "$MASTER_BRANCH" ] || [ "$target_branch" = "origin/$MASTER_BRANCH" ]; then
        log_error "FORBIDDEN: Pushing directly to $MASTER_BRANCH is not allowed!"
        log_error "Use PR workflow instead: create branch, PR, wait for AI review, auto-merge"
        return 1
    fi

    if [[ "$target_branch" == *"$MASTER_BRANCH"* ]]; then
        log_error "FORBIDDEN: Pushing to $MASTER_BRANCH branch is not allowed!"
        return 1
    fi

    return 0
}

# Pre-push enforcement
pre_push() {
    log_info "=== AIOS Workflow Enforcer: Pre-Push Check ==="

    local current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
    local push_target="${1:-origin}"

    log_info "Current branch: $current_branch"
    log_info "Push target: $push_target"

    # Check if pushing to master
    if ! check_master_push "$current_branch"; then
        log_error "Aborting push due to workflow violation"
        return 1
    fi

    # Verify branch follows naming convention
    if [[ "$current_branch" != feat/* ]] && [[ "$current_branch" != fix/* ]] && \
       [[ "$current_branch" != chore/* ]] && [[ "$current_branch" != docs/* ]] && \
       [[ "$current_branch" != refactor/* ]]; then
        log_warn "Branch name '$current_branch' doesn't follow conventional naming"
        log_warn "Recommended: feat/*, fix/*, chore/*, docs/*, refactor/*"
    fi

    log_success "Pre-push check passed - no workflow violations"
    return 0
}

# Post-merge cleanup
post_merge() {
    local pr_number="${1:-}"

    if [ -z "$pr_number" ]; then
        log_error "Usage: enforce-workflow.sh post-merge <pr_number>"
        return 1
    fi

    log_info "=== AIOS Workflow Enforcer: Post-Merge PR #$pr_number ==="

    # Get PR info
    local pr_info
    pr_info=$(gh pr view "$pr_number" --json state,title,headRefName,body,mergedAt 2>/dev/null) || {
        log_error "PR #$pr_number not found or API error"
        return 1
    }

    local pr_state=$(echo "$pr_info" | jq -r '.state')
    local pr_title=$(echo "$pr_info" | jq -r '.title')
    local head_branch=$(echo "$pr_info" | jq -r '.headRefName')
    local pr_body=$(echo "$pr_info" | jq -r '.body // empty')

    # Check if PR is actually merged
    if [ "$pr_state" != "MERGED" ]; then
        log_warn "PR #$pr_number is not merged yet (state: $pr_state)"
        return 1
    fi

    log_success "PR #$pr_number is merged: $pr_title"

    # Extract issue number from PR body (e.g., "Closes #42", "Fixes #42")
    local issue_match
    issue_match=$(echo "$pr_body" | grep -oE '[Cc]loses #?[0-9]+|[Ff]ixes #?[0-9]+|[Rr]esolves #?[0-9]+' | head -1 || true)

    if [ -n "$issue_match" ]; then
        local issue_number
        issue_number=$(echo "$issue_match" | grep -oE '[0-9]+')
        log_info "Found issue reference: #$issue_number"

        # Check if issue is already closed
        local issue_state
        issue_state=$(gh issue view "$issue_number" --json state 2>/dev/null | jq -r '.state')

        if [ "$issue_state" = "CLOSED" ]; then
            log_success "Issue #$issue_number is already closed"
        else
            log_info "Closing issue #$issue_number..."
            if gh issue close "$issue_number" --comment "Closed by PR #$pr_number merge" 2>/dev/null; then
                log_success "Issue #$issue_number closed"
            else
                log_warn "Could not close issue #$issue_number (may already be closed)"
            fi
        fi
    else
        log_warn "No issue close reference found in PR body"
    fi

    # Cleanup the merged branch
    cleanup_branch "$head_branch"

    # Also cleanup any feat/auto-* branches for this issue
    if [ -n "$issue_number" ]; then
        local auto_branch="feat/auto-issue-$issue_number"
        if git ls-remote --heads origin "$auto_branch" 2>/dev/null | grep -q "$auto_branch"; then
            log_info "Found auto-generated branch $auto_branch, cleaning up..."
            cleanup_branch "$auto_branch"
        fi
    fi

    log_success "Post-merge cleanup complete"
    return 0
}

# Cleanup a single branch
cleanup_branch() {
    local branch_name="$1"

    if [ -z "$branch_name" ]; then
        return 0
    fi

    log_info "Cleaning up branch: $branch_name"

    # Delete remote branch
    if git ls-remote --heads origin "$branch_name" 2>/dev/null | grep -q "$branch_name"; then
        log_info "Deleting remote branch: origin/$branch_name"
        if git push origin --delete "$branch_name" 2>/dev/null; then
            log_success "Deleted remote branch: $branch_name"
        else
            log_warn "Failed to delete remote branch $branch_name"
        fi
    else
        log_info "Remote branch $branch_name does not exist"
    fi

    # Delete local branch if exists
    if git show-ref --verify --quiet "refs/heads/$branch_name"; then
        log_info "Deleting local branch: $branch_name"
        if git branch -D "$branch_name" 2>/dev/null; then
            log_success "Deleted local branch: $branch_name"
        else
            log_warn "Failed to delete local branch $branch_name"
        fi
    else
        log_info "Local branch $branch_name does not exist"
    fi
}

# Cleanup all merged branches
cleanup_merged_branches() {
    log_info "=== AIOS Workflow Enforcer: Cleanup Merged Branches ==="

    # Get all merged PR head branches
    local merged_prs
    merged_prs=$(gh pr list --state merged --json number,headRefName 2>/dev/null | jq -r '.[] | "\(.number) \(.headRefName)"') || {
        log_warn "Could not fetch merged PRs"
        return 0
    }

    if [ -z "$merged_prs" ]; then
        log_warn "No merged PRs found"
        return 0
    fi

    local count=0
    while read -r pr_num branch_name; do
        if [ -n "$branch_name" ]; then
            cleanup_branch "$branch_name"
            count=$((count + 1))
        fi
    done <<< "$merged_prs"

    log_success "Cleaned up $count merged branches"

    # Prune remote tracking refs
    git remote prune origin 2>/dev/null || true

    return 0
}

# Verify no direct master push in recent commits
audit_master() {
    log_info "=== AIOS Workflow Enforcer: Audit Master Branch ==="

    local local_sha
    local_sha=$(git rev-parse master 2>/dev/null || echo "")
    local remote_sha
    remote_sha=$(git rev-parse origin/master 2>/dev/null || echo "")

    if [ -z "$local_sha" ] || [ -z "$remote_sha" ]; then
        log_warn "Could not determine master SHA"
        return 0
    fi

    if [ "$local_sha" != "$remote_sha" ]; then
        local master_branch
        master_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

        if [ "$master_branch" = "master" ]; then
            local commits_ahead
            commits_ahead=$(git rev-list --count origin/master..master 2>/dev/null || echo "0")

            if [ "$commits_ahead" -gt 0 ]; then
                log_error "DANGER: You have $commits_ahead unpushed commits on master!"
                log_error "These should be pushed via PR workflow, not directly!"
                log_error "Commits on master must go through PR review process"
                return 1
            fi
        fi
    fi

    log_success "No direct master push violations detected"
    return 0
}

# Show help
show_help() {
    cat << 'EOF'
AIOS Workflow Enforcer

Usage: enforce-workflow.sh <command> [args]

Commands:
  pre-push [branch]          Check before git push (default: current branch)
  post-merge <pr_number>     Cleanup after PR merge (closes issue, deletes branch)
  close-issue <issue_num>    Close an issue
  cleanup-branches           Cleanup all merged branches (local and remote)
  check-master-push [branch] Check if pushing to master
  audit                      Audit master branch for direct pushes
  help                       Show this help

Examples:
  # Before any push
  ./enforce-workflow.sh pre-push

  # After PR is merged
  ./enforce-workflow.sh post-merge 42

  # Cleanup all merged branches
  ./enforce-workflow.sh cleanup-branches

  # Audit for violations
  ./enforce-workflow.sh audit

Rules Enforced:
  - NEVER push directly to master
  - NEVER run gh pr merge manually
  - MUST close issue when PR merges
  - MUST delete merged branches
  - MUST use PR workflow for all changes

EOF
}

# Main command dispatcher
main() {
    local command="${1:-}"

    case "$command" in
        pre-push)
            pre_push "${2:-}"
            ;;
        post-merge)
            post_merge "${2:-}"
            ;;
        close-issue)
            local issue_num="${2:-}"
            if [ -z "$issue_num" ]; then
                log_error "Usage: enforce-workflow.sh close-issue <issue_number>"
                exit 1
            fi
            if gh issue close "$issue_num" --comment "Closed via workflow enforcer" 2>/dev/null; then
                log_success "Issue #$issue_num closed"
            else
                log_warn "Issue $issue_num may already be closed or not found"
            fi
            ;;
        cleanup-branches)
            cleanup_merged_branches
            ;;
        check-master-push)
            check_master_push "${2:-}"
            ;;
        audit)
            audit_master
            ;;
        help|--help|-h)
            show_help
            ;;
        "")
            log_error "No command specified"
            show_help
            exit 1
            ;;
        *)
            log_error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

main "$@"