#!/usr/bin/env bash
# Prepare an upstream integration PR, never merge it.
set -euo pipefail
tag="$1"
git check-ref-format "refs/tags/$tag"
git fetch --no-tags origin refs/heads/integration/desktop:refs/remotes/origin/integration/desktop
git fetch --no-tags https://github.com/Flow-Tube/Flow-Desktop.git "refs/tags/$tag:refs/flow-upstream/selected"
source="$(git rev-parse --verify 'refs/flow-upstream/selected^{commit}')"
base="$(git rev-parse refs/remotes/origin/integration/desktop)"
git switch --detach "$base"
if [[ -f FLOW_DESKTOP_REVISION ]]; then
  old="$(tr -d '\r\n' < FLOW_DESKTOP_REVISION)"
  [[ "$old" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid recorded upstream revision" >&2; exit 1; }
  if [[ "$old" == "$source" ]]; then
    echo "Flow Desktop $tag already recorded."
    exit 0
  fi
  git merge-base --is-ancestor "$old" "$source" || {
    echo "Upstream history diverged or selected release is older; reconcile manually." >&2
    exit 1
  }
fi
branch="update/desktop-${source:0:12}-${base:0:8}"
if git ls-remote --exit-code --heads origin "refs/heads/$branch" >/dev/null; then
  echo "Existing update branch: $branch"
else
  # Only Git operations run after fetching upstream; do not execute upstream code with a write token.
  if ! git -c core.hooksPath=/dev/null merge --no-ff --no-edit "$source"; then
    git diff --name-only --diff-filter=U >&2
    git merge --abort || true
    echo "Upstream merge requires manual conflict resolution." >&2
    exit 1
  fi
  printf '%s\n' "$source" > FLOW_DESKTOP_REVISION
  git add FLOW_DESKTOP_REVISION
  if ! git diff --cached --quiet; then
    git -c core.hooksPath=/dev/null commit -m "chore: record Flow Desktop $tag"
  fi
  git push origin "HEAD:refs/heads/$branch"
fi
existing="$(gh pr list --repo "$GITHUB_REPOSITORY" --state open --base integration/desktop --head "$branch" --json number --jq '.[0].number // empty')"
if [[ -z "$existing" ]]; then
  gh pr create --repo "$GITHUB_REPOSITORY" --draft --base integration/desktop --head "$branch" \
    --title "Update Flow Desktop to $tag" \
    --body "Upstream revision: $source. Integration base: $base. Review and test before merging manually."
else
  echo "Existing integration PR: #$existing"
fi
