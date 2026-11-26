#!/usr/bin/env bash
# Local developer launcher. Production deployments use compose.web.yml.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
configure=0
update=0
for arg in "$@"; do
  case "$arg" in
    --configure) configure=1 ;;
    --update) update=1 ;;
    --help) echo "Usage: bash scripts/web-dev.sh [--configure] [--update]"; exit 0 ;;
    *) echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done
if (( update )); then
  git diff --quiet && git diff --cached --quiet || { echo "Commit or stash local changes before updating." >&2; exit 1; }
  branch="$(git branch --show-current)"
  if [[ -n "$branch" && "$branch" != integration/desktop ]]; then
    echo "Update requires integration/desktop or a detached web checkout." >&2; exit 1
  fi
  git fetch origin integration/desktop
  git merge-base --is-ancestor HEAD FETCH_HEAD || { echo "Local history diverged; refusing to replace it." >&2; exit 1; }
  if [[ -n "$branch" ]]; then git merge --ff-only FETCH_HEAD; else git switch --detach FETCH_HEAD; fi
  if (( configure )); then exec bash "$root/scripts/web-dev.sh" --configure; fi
  exec bash "$root/scripts/web-dev.sh"
fi
command -v pnpm >/dev/null || { echo "Install Node and pnpm first (see docs/WEB.md)." >&2; exit 1; }
command -v cargo >/dev/null || { echo "Install Rust first (see docs/WEB.md)." >&2; exit 1; }
config="${FLOW_WEB_DEV_CONFIG:-${XDG_CONFIG_HOME:-$HOME/.config}/flow-web/dev.conf}"
password="${FLOW_WEB_PASSWORD:-}"
bind="${FLOW_WEB_BIND:-127.0.0.1:8780}"
origin="${FLOW_WEB_ORIGIN:-http://127.0.0.1:8780}"
sync_url="${FLOW_WEB_SYNC_URL:-}"
if [[ -f "$config" ]]; then
  # Read literal values, never execute a configuration file as shell code.
  mapfile -t saved < "$config"
  if (( ${#saved[@]} != 4 )); then echo "Invalid config: $config" >&2; exit 1; fi
  password="${saved[0]}"
  bind="${saved[1]}"
  origin="${saved[2]}"
  sync_url="${saved[3]}"
fi
if (( configure )) || [[ ! -f "$config" ]]; then
  [[ -t 0 ]] || { echo "Run interactively once to configure Flow Web." >&2; exit 1; }
  read -r -s -p "Password (16+ characters; blank keeps existing): " entered; echo
  password="${entered:-$password}"
  if (( ${#password} < 16 )); then echo "Password must be at least 16 characters." >&2; exit 1; fi
  read -r -p "Bind address [$bind]: " entered; bind="${entered:-$bind}"
  read -r -p "Browser origin [$origin]: " entered; origin="${entered:-$origin}"
  read -r -p "Flow Sync server URL [$sync_url] (enter - to clear): " entered
  if [[ "$entered" == "-" ]]; then sync_url=""; else sync_url="${entered:-$sync_url}"; fi
  umask 077
  mkdir -p -- "$(dirname -- "$config")"
  temp="$(mktemp "${config}.XXXXXX")"
  trap 'rm -f -- "$temp"' EXIT
  printf '%s\n' "$password" "$bind" "$origin" "$sync_url" > "$temp"
  mv -f -- "$temp" "$config"
  trap - EXIT
fi
export FLOW_WEB_PASSWORD="$password" FLOW_WEB_BIND="$bind" FLOW_WEB_ORIGIN="$origin" FLOW_WEB_SYNC_URL="$sync_url"
export FLOW_WEB_DIRECT_MEDIA="${FLOW_WEB_DIRECT_MEDIA:-1}"
# Keep the existing web-server/data location so local databases and TOTP survive updates.
pnpm install --frozen-lockfile
pnpm build --mode web
cd web-server
echo "Flow Web: $origin"
exec cargo run --locked
