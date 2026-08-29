#!/bin/sh
# Removes build output and this app's local state so the next run is a first run:
# onboarding shows again, no trusted phone, fresh TLS certificate.
#
# Everything removed under ~/Library is scoped to our bundle identifiers. Never
# match on product name, because a directory called "Caddie" or "Companion" may
# belong to something else entirely.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
IDS="dev.caddie.desktop dev.deskthing.desktop"

DEEP=0
FORCE=0
DRY=0
for arg in "$@"; do
  case "$arg" in
    --deep) DEEP=1 ;;
    --force) FORCE=1 ;;
    -n | --dry-run) DRY=1 ;;
    -h | --help)
      echo "usage: pnpm purge [--deep] [--force] [--dry-run]"
      echo "  --deep     also remove the Rust target directory (slow rebuild)"
      echo "  --force    quit a running desktop app instead of refusing"
      echo "  --dry-run  print what would be removed and stop"
      exit 0
      ;;
    *)
      echo "purge: unknown option $arg" >&2
      exit 2
      ;;
  esac
done

if pgrep -f "companion-desktop" >/dev/null 2>&1; then
  if [ "$FORCE" -eq 1 ] && [ "$DRY" -eq 0 ]; then
    echo "purge: stopping the running desktop app"
    pkill -f "companion-desktop" || true
    sleep 1
  elif [ "$DRY" -eq 0 ]; then
    echo "purge: the desktop app is running. Quit it, or pass --force." >&2
    exit 1
  fi
fi

removed=0
drop() {
  target=$1
  [ -n "$target" ] || return 0
  [ -e "$target" ] || return 0
  size=$(du -sh "$target" 2>/dev/null | cut -f1)
  if [ "$DRY" -eq 1 ]; then
    printf '  would remove  %-7s %s\n' "$size" "$target"
    return 0
  fi
  rm -rf "$target"
  printf '  removed       %-7s %s\n' "$size" "$target"
  removed=$((removed + 1))
}

echo "app state"
for id in $IDS; do
  drop "$HOME/Library/Application Support/$id"
  drop "$HOME/Library/Caches/$id"
  drop "$HOME/Library/Logs/$id"
  drop "$HOME/Library/WebKit/$id"
  drop "$HOME/Library/HTTPStorages/$id"
  drop "$HOME/Library/Saved Application State/$id.savedState"
  drop "$HOME/Library/Preferences/$id.plist"
  drop "$HOME/Library/LaunchAgents/$id.plist"
done

echo "build output"
drop "$ROOT/apps/desktop/dist"
drop "$ROOT/apps/pwa/dist"
drop "$ROOT/node_modules/.vite"
drop "$ROOT/apps/pwa/node_modules/.vite"
drop "$ROOT/apps/desktop/node_modules/.vite"

if [ "$DEEP" -eq 1 ]; then
  echo "rust build"
  drop "$ROOT/apps/desktop/src-tauri/target"
else
  if [ -d "$ROOT/apps/desktop/src-tauri/target" ]; then
    size=$(du -sh "$ROOT/apps/desktop/src-tauri/target" 2>/dev/null | cut -f1)
    echo "rust build"
    printf '  kept          %-7s target, pass --deep to remove it\n' "$size"
  fi
fi

if [ "$DRY" -eq 1 ]; then
  echo
  echo "dry run, nothing was removed"
  exit 0
fi

echo
echo "$removed paths removed. Next pnpm dev is a first run:"
echo "  onboarding shows again"
echo "  the phone must pair again, and will see a new certificate fingerprint"
echo "  on the phone, close the web app fully and reopen it so it drops its cache"
