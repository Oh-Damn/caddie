#!/bin/sh
# Signs the bundled macOS app so TCC can hold on to its Accessibility and
# Automation grants. Tauri leaves the bundle unsigned, which means the only
# signature is the linker's ad-hoc one on the Mach-O: the bundle Info.plist is
# not bound, resources are not sealed, and the signing identifier is a
# per-build crate hash. macOS records a grant against that identifier, so every
# rebuild silently invalidates it and Accessibility reads back as off even
# though the toggle in System Settings is still on.
#
# Set CADDIE_SIGNING_IDENTITY to a stable identity so grants survive rebuilds.
# scripts/signing-identity.sh creates a local one. Without it this falls back to
# ad-hoc, which fixes the current build but has to be re-granted after the next.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
app=""
for candidate in \
  "$root/apps/desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/Caddie.app" \
  "$root/apps/desktop/src-tauri/target/release/bundle/macos/Caddie.app" \
  "$root/apps/desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Caddie.app" \
  "$root/apps/desktop/src-tauri/target/x86_64-apple-darwin/release/bundle/macos/Caddie.app"; do
  if [ -d "$candidate" ]; then
    app="$candidate"
    break
  fi
done

if [ $# -gt 0 ]; then
  app="$1"
fi

if [ -z "$app" ]; then
  echo "No Caddie.app found. Build it first: pnpm build:app" >&2
  exit 1
fi

identity="${CADDIE_SIGNING_IDENTITY:-${APPLE_SIGNING_IDENTITY:--}}"
if [ "$identity" = "-" ]; then
  echo "Signing ad-hoc. Grants will not survive the next rebuild."
  echo "Run scripts/signing-identity.sh once for an identity that does."
else
  echo "Signing with $identity"
fi

entitlements="$root/apps/desktop/src-tauri/Entitlements.plist"

# Without --options runtime plus the entitlements, a re-sign here drops the
# Apple Events entitlement the bundler applied and Automation stops working.
codesign --force --deep --sign "$identity" --timestamp=none \
  --options runtime --entitlements "$entitlements" "$app"
codesign --verify --strict "$app" 2>/dev/null || true
codesign -dvv "$app" 2>&1 | grep -E "^(Identifier|CDHash|TeamIdentifier|Sealed)"

echo
echo "Signed $app"
echo "The old grants are stale. Clear and re-grant them:"
echo "  tccutil reset Accessibility dev.caddie.desktop"
echo "  tccutil reset AppleEvents dev.caddie.desktop"
