#!/bin/sh
# Points the cask at a published release: downloads the dmg, checksums it, and
# rewrites version and sha256 in place.
#
#   sh update-cask.sh 0.1.0 owner/repo
set -eu

version="${1:-}"
slug="${2:-}"
if [ -z "$version" ] || [ -z "$slug" ]; then
  echo "usage: sh update-cask.sh <version> <owner/repo>" >&2
  exit 2
fi

cask=$(cd "$(dirname "$0")" && pwd)/Casks/caddie.rb
url="https://github.com/$slug/releases/download/v$version/Caddie_${version}_universal.dmg"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "Fetching $url"
if ! curl -fsSL --retry 3 -o "$tmp/caddie.dmg" "$url"; then
  echo "Could not download the release asset. Is the release published and the" >&2
  echo "dmg attached to it?" >&2
  exit 1
fi

sha=$(shasum -a 256 "$tmp/caddie.dmg" | cut -d' ' -f1)
echo "sha256 $sha"

# Only the two lines that pin the release; leave everything else untouched.
sed -i '' \
  -e "s|^  version \".*\"|  version \"$version\"|" \
  -e "s|^  sha256 \".*\"|  sha256 \"$sha\"|" \
  "$cask"

echo "Updated $cask"
grep -E '^  (version|sha256) ' "$cask"
