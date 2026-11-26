#!/usr/bin/env bash
# Generate a new candidate branch from the trusted integration tree.
set -euo pipefail
git fetch --no-tags origin refs/heads/main:refs/remotes/origin/main refs/heads/integration/desktop:refs/remotes/origin/integration/desktop
base="$(git rev-parse refs/remotes/origin/main)"
source="$(git rev-parse refs/remotes/origin/integration/desktop)"
if ! git cat-file -e "$base:UPSTREAM.lock" 2>/dev/null; then
  echo "Initial migration is not merged yet; use the refactor/web-only PR."
  exit 0
fi
git switch --detach "$source"
tree="$(python3 scripts/export-web.py "$source")"
if [[ "$tree" == "$(git rev-parse "$base^{tree}")" ]]; then
  echo "main already matches integration/desktop."
  exit 0
fi
branch="web-export/${source:0:12}-${base:0:8}"
if ! git ls-remote --exit-code --heads origin "refs/heads/$branch" >/dev/null; then
  candidate="$(git commit-tree "$tree" -p "$base" -p "$source" -m "chore: export Flow Web")"
  python3 scripts/export-web.py "$source" --verify "$candidate"
  git push origin "$candidate:refs/heads/$branch"
fi
existing="$(gh pr list --repo "$GITHUB_REPOSITORY" --state open --base main --head "$branch" --json number --jq '.[0].number // empty')"
if [[ -z "$existing" ]]; then
  gh pr create --repo "$GITHUB_REPOSITORY" --draft --base main --head "$branch" \
    --title "Export Flow Web (${source:0:12})" \
    --body "Generated from integration commit $source. Main base: $base. Review and test before merging manually."
else
  echo "Existing web export PR: #$existing"
fi
# Token-generated pushes may not trigger ordinary CI.
gh workflow run web-check.yml --repo "$GITHUB_REPOSITORY" --ref "$branch"
