#!/bin/sh
# Creates a local self-signed code signing identity, once. Signing with it gives
# the app a designated requirement that does not change when the binary does, so
# a granted Accessibility or Automation permission survives rebuilds. An Apple
# Developer ID certificate does the same and is what a shipped build wants.
#
# This writes to the login keychain and macOS will ask for the login password.
set -eu

name="${1:-Caddie Dev}"

if security find-certificate -c "$name" >/dev/null 2>&1; then
  echo "\"$name\" already exists. Use it with:"
  echo "  export CADDIE_SIGNING_IDENTITY=\"$name\""
  exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/req.cnf" <<EOF
[req]
distinguished_name=dn
prompt=no
x509_extensions=v3
[dn]
CN=$name
[v3]
basicConstraints=critical,CA:false
keyUsage=critical,digitalSignature
extendedKeyUsage=critical,codeSigning
EOF

openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout "$work/key.pem" -out "$work/cert.pem" -config "$work/req.cnf" >/dev/null 2>&1
openssl pkcs12 -export -inkey "$work/key.pem" -in "$work/cert.pem" \
  -out "$work/id.p12" -passout pass:caddie -name "$name" >/dev/null 2>&1

security import "$work/id.p12" -k "$HOME/Library/Keychains/login.keychain-db" \
  -P caddie -T /usr/bin/codesign
security add-trusted-cert -d -r trustRoot -p codeSign \
  -k "$HOME/Library/Keychains/login.keychain-db" "$work/cert.pem"

echo
echo "Created \"$name\". Build signed with it:"
echo "  CADDIE_SIGNING_IDENTITY=\"$name\" pnpm build:app"
echo
echo "Remove it later with:"
echo "  security delete-identity -c \"$name\""
