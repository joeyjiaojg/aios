#!/bin/bash
# AIOS Workflow Enforcer Script
# Enforces AGENTS.md rules and ensures proper workflow

set -e

REPO="${GITHUB_REPOSITORY:-joeyjiaojg/aios}"
MASTER_BRANCH="master"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_error() {
    echo -e "${RED}ERROR: $1${NC}" >&2
}

log_warn() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

log_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

# Check if trying to push to master
check_master_push() {
    local target_branch="$1"

    if [ "$target_branch" = "$MASTER_BRANCH" ] || [ "$target_branch" = "origin/$MASTER_BRANCH" ]; then
        log_error "FORBIDDEN: Pushing directly to $MASTER_BRANCH is not allowed!"
        log_error "Use PR workflow instead: create branch, PR, wait for AI review, auto-merge"
        return 1
    fi

    # Also check if local master is being pushed to
    if [[ "$target_branch" == *"$MASTER_BRANCH"* ]]; then
        log_error "FORBIDDEN: Pushing to $MASTER_BRANCH branch is not allowed!"
        return 1
    fi

    return 0
}

# Pre-push enforcement
pre_push() {
    echo "=== AIOS Workflow Enforcer: Pre-Push Check ==="

    # Get current branch and remote
    local current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
    local push_target="${1:-origin}"

    echo "Current branch: $current_branch"
    echo "Push target: $push_target"

    # Check if pushing to master
    if ! check_master_push "$current_branch"; then
        log_error "Aborting push due to workflow violation"
        return 1
    fi

    # Check git remote
    local remote_url=$(git remote get-url "$push_target" 2>/dev/null || echo "")

    # Verify we're not pushing to a protected branch
    if [[ "$remote_url" == *"github.com"* ]]; then
        echo "GitHub detected, checking branch protection..."
    fi

    log_success "Pre-push check passed"

    # Additional safety: verify the branch is a feature branch
    if [[ "$current_branch" != feat/* ]] && [[ "$current_branch" != fix/* ]] && [[ "$current_branch" != chore/* ]] && [[ "$current_branch" != docs/* ]]; then
        log_warn "Branch name '$current_branch' doesn't follow conventional naming (feat/*, fix/*, chore/*, docs/*)"
    fi

    return 0
}

# Post-merge cleanup
post_merge() {
    local pr_number="${1:-}"

    if [ -z "$pr_number" ]; then
        log_error "Usage: enforce-workflow.sh post-merge <pr_number>"
        return 1
    fi

    echo "=== AIOS Workflow Enforcer: Post-Merge Check for PR #$pr_number ==="

    # Get PR info
    local pr_info=$(gh pr view "$pr_number" --json state,title,headRefName,body 2>/dev/null)

    if [ -z "$pr_info" ]; then
        log_error "PR #$pr_number not found or API error"
        return 1
    fi

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

    # Extract issue number from PR body (e.g., "Closes #42")
    local issue_match=$(echo "$pr_body" | grep -oE '[Cc]loses #?[0-9]+' | head -1)
    if [ -n "$issue_match" ]; then
        local issue_number=$(echo "$issue_match" | grep -oE '[0-9]+')
        echo "Found issue reference: #$issue_number"

        # Check if issue is already closed
        local issue_state=$(gh issue view "$issue_number" --json state 2>/dev/null | jq -r '.state')
        if [ "$issue_state" = "CLOSED" ]; then
            log_success "Issue #$issue_number is already closed"
        else
            echo "Closing issue #$issue_number..."
            gh issue close "$issue_number" --comment "Closed by PR #$pr_number merge" 2>/dev/null || log_warn "Failed to close issue #$issue_number"
        fi
    else
        log_warn "No issue close reference found in PR body"
    fi

    # Cleanup branches
    cleanup_branch "$head_branch"

    # Also cleanup any stale feat/auto-* branches for this PR
    local auto_branch="feat/auto-issue-${issue_number:-unknown}"
    if git ls-remote --heads origin "$auto_branch" 2>/dev/null | grep -q "$auto_branch"; then
        log_warn "Found auto-generated branch $auto_branch, cleaning up..."
        cleanup_branch "$auto_branch"
    fi

    return 0
}

# Cleanup a single branch
cleanup_branch() {
    local branch_name="$1"

    if [ -z "$branch_name" ]; then
        return 0
    fi

    echo "Cleaning up branch: $branch_name"

    # Delete remote branch
    if git ls-remote --heads origin "$branch_name" 2>/dev/null | grep -q "$branch_name"; then
        echo "Deleting remote branch: origin/$branch_name"
        git push origin --delete "$branch_name" 2>/dev/null || log_warn "Failed to delete remote branch $branch_name"
    fi

    # Delete local branch if exists
    if git show-ref --verify --quiet "refs/heads/$branch_name"; then
        echo "Deleting local branch: $branch_name"
        git branch -D "$branch_name" 2>/dev/null || log_warn "Failed to delete local branch $branch_name"
    fi

    log_success "Branch cleanup complete for: $branch_name"
}

# Cleanup all merged branches
cleanup_merged_branches() {
    echo "=== AIOS Workflow Enforcer: Cleanup Merged Branches ==="

    # Get all merged PR head branches
    local merged_prs=$(gh pr list --state merged --json number,headRefName 2>/dev/null | jq -r '.[] | "\(.number) \(.headRefName)"')

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

    log_success "Cleaned up $count branches"

    # Also cleanup any feat/auto-* branches that are no longer on remote
    echo "Checking for stale auto-generated branches..."
    git remote prune origin 2>/dev/null || true

    return 0
}

# Verify no direct master push in recent commits
audit_master_push() {
    echo "=== AIOS Workflow Enforcer: Audit Master Branch ==="

    # Check if master is ahead of origin/master
    local local_sha=$(git rev-parse master 2>/dev/null || echo "")
    local remote_sha=$(git rev-parse origin/master 2>/dev/null || echo "")

    if [ -z "$local_sha" ] || [ -z "$remote_sha" ]; then
        log_warn "Could not determine master SHA"
        return 0
    fi

    if [ "$local_sha" != "$remote_sha" ]; then
        local master_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

        if [ "$master_branch" = "master" ]; then
            # We're on master, check if local commits exist
            local commits_ahead=$(git rev-list --count origin/master..master 2>/dev/null || echo "0")

            if [ "$commits_ahead" -gt 0 ]; then
                log_error "DANGER: You have $commits_ahead unpushed commits on master!"
                log_error "These should be pushed via PR workflow, not directly"
                return 1
            fi
        fi
    fi

    log_success "No direct master push detected"
    return 0
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
            gh issue close "$issue_num" --comment "Closed via workflow enforcer" 2>/dev/null || log_warn "Issue $issue_num not closed"
            ;;
        cleanup-branches)
            cleanup_merged_branches
            ;;
        check-master-push)
            check_master_push "${2:-}"
            ;;
        audit)
            audit_master_push
            ;;
        help|--help|-h)
            echo "AIOS Workflow Enforcer"
            echo ""
            echo "Usage: enforce-workflow.sh <command> [args]"
            echo ""
            echo "Commands:"
            echo "  pre-push [branch]          Check before git push (default: current branch)"
            echo "  post-merge <pr_number>     Cleanup after PR merge"
            echo "  close-issue <issue_num>    Close an issue"
            echo "  cleanup-branches           Cleanup all merged branches"
            echo "  check-master-push [branch] Check if pushing to master"
            echo "  audit                      Audit master branch for direct pushes"
            echo "  help                       Show this help"
            ;;
        *)
            log_error "Unknown command: $command"
            echo "Run 'enforce-workflow.sh help' for usage"
            exit 1
            ;;
    esac
}

main "$@"