#!/bin/bash
# Setup branch protection rules for AIOS
# Run once to enforce PR-only workflow on GitHub

set -e

REPO="${1:-joeyjiaojg/aios}"
BRANCH="master"

echo "Setting up branch protection for $REPO:$BRANCH..."

gh api \
  --method PUT \
  /repos/$REPO/branches/$BRANCH/protection \
  --input - <<'EOF'
{
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 1
  },
  "required_status_checks": {
    "strict": true,
    "contexts": ["ai-review"]
  },
  "enforce_admins": true,
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": true,
  "lock_branch": false,
  "allow_fork_syncing": false
}
EOF

echo "Done! PR-only workflow enforced on $BRANCH."
