#!/bin/sh
# Builds a signed Apple Silicon (arm64) macOS bundle.
#
# Same job as build-app.sh, minus the x86_64 slice. The universal build has to
# compile the whole dependency tree twice and lipo the results, so on an M chip
# this is roughly half the work for a bundle that runs natively on the machine
# that built it. Use build-app.sh for anything handed to an Intel Mac.
#
# Signing happens here rather than after the fact, because the dmg is bundled
# from the app and would otherwise carry an unsigned copy. macOS keys an
# Accessibility or Automation grant to the signing identifier, and Tauri's
# unsigned default leaves that as a per-build crate hash, so every rebuild
# invalidates the grant while the toggle in System Settings still looks on.
#
# Set CADDIE_SIGNING_IDENTITY to a stable identity so grants survive rebuilds.
# scripts/signing-identity.sh creates a local one. The ad-hoc default fixes the
# build in hand but has to be re-granted after the next one.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
target=aarch64-apple-darwin
identity="${CADDIE_SIGNING_IDENTITY:-${APPLE_SIGNING_IDENTITY:--}}"

if [ "$(uname -m)" != "arm64" ]; then
  echo "This builds an arm64-only bundle and cross-compiling needs the Apple" >&2
  echo "Silicon toolchain. On Intel use: pnpm build:app" >&2
  exit 1
fi

if ! sh "$root/scripts/with-cargo.sh" rustup target list --installed 2>/dev/null |
  grep -qx "$target"; then
  echo "Rust target $target is missing. Add it with:" >&2
  echo "  rustup target add $target" >&2
  exit 1
fi

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
  pnpm --dir "$root" --filter @companion/desktop tauri build --target "$target"

app="$root/apps/desktop/src-tauri/target/$target/release/bundle/macos/Caddie.app"
codesign --verify --strict --deep "$app"
echo
lipo -archs "$app/Contents/MacOS/companion-desktop"
codesign -dvv "$app" 2>&1 | grep -E "^(Identifier|CDHash|TeamIdentifier|Sealed)"

echo
echo "Built $app"
dmg=$(ls "$root/apps/desktop/src-tauri/target/$target/release/bundle/dmg/"*.dmg 2>/dev/null | head -1) || true
if [ -n "${dmg:-}" ]; then
  echo "       $dmg"
fi
echo
echo "The grants from the previous build no longer match. Clear them, then"
echo "grant again on next launch:"
echo "  tccutil reset Accessibility dev.caddie.desktop"
echo "  tccutil reset AppleEvents dev.caddie.desktop"
