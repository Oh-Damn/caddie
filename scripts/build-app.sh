#!/bin/sh
# Builds the universal macOS bundle with a code signature.
#
# Tauri leaves the bundle unsigned unless an identity is set, and the only
# signature is then the linker's ad-hoc one on the Mach-O: the Info.plist is not
# bound, resources are not sealed, and the signing identifier is a per-build
# crate hash. macOS keys an Accessibility or Automation grant to that
# identifier, so every rebuild invalidates it and the permission reads back as
# off while the toggle in System Settings still looks on.
#
# Signing has to happen here rather than after the fact, because the dmg is
# bundled from the app and would otherwise carry an unsigned copy.
#
# Set CADDIE_SIGNING_IDENTITY to a stable identity so grants survive rebuilds.
# scripts/signing-identity.sh creates a local one. The ad-hoc default fixes the
# build in hand but has to be re-granted after the next one.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
identity="${CADDIE_SIGNING_IDENTITY:-${APPLE_SIGNING_IDENTITY:--}}"

if [ "$identity" = "-" ]; then
  echo "Signing ad-hoc. Grants will not survive the next rebuild."
  echo "Run scripts/signing-identity.sh once for an identity that does."
else
  echo "Signing with $identity"
fi

# bundle_dmg.sh mounts its staging volume as "Caddie" and fails outright if an
# older installer is still mounted under that name.
for vol in /Volumes/Caddie /Volumes/dmg.*; do
  if [ -d "$vol" ]; then
    echo "Detaching stale volume $vol"
    hdiutil detach "$vol" -quiet || true
  fi
done

APPLE_SIGNING_IDENTITY="$identity" \
  pnpm --dir "$root" --filter @companion/desktop tauri build --target universal-apple-darwin

app="$root/apps/desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/Caddie.app"
codesign --verify --strict --deep "$app"
echo
lipo -archs "$app/Contents/MacOS/companion-desktop"
codesign -dvv "$app" 2>&1 | grep -E "^(Identifier|CDHash|TeamIdentifier|Sealed)"

echo
echo "Built $app"
echo "The grants from the previous build no longer match. Clear them, then"
echo "grant again on next launch:"
echo "  tccutil reset Accessibility dev.caddie.desktop"
echo "  tccutil reset AppleEvents dev.caddie.desktop"
