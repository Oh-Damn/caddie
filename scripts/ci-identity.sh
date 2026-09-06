#!/bin/sh
# Creates a self-signed code signing identity for CI, as a .p12 plus a random
# password, and prints how to load both into GitHub secrets.
#
# Why this exists: macOS keys Accessibility and Automation grants, and firewall
# decisions, to an app's code signature. An ad-hoc signature is a fresh identity
# on every build, so each release silently revokes what the user granted the
# last one. Signing every build with one stable certificate keeps those grants
# across updates. It does not make macOS trust the app — that needs a Developer
# ID — it only stops the identity from moving.
#
# The private key never leaves this machine except as a GitHub secret. Nothing
# is written into the repository.
set -eu

name="${1:-Caddie CI}"
out="${2:-$HOME/.caddie-ci-identity}"

mkdir -p "$out"
chmod 700 "$out"

if [ -f "$out/identity.p12" ]; then
  echo "An identity already exists at $out/identity.p12"
  echo "Delete it first if you mean to replace it. Replacing it resets every"
  echo "grant your users have given the app."
  exit 1
fi

pass=$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 32)

cat > "$out/req.cnf" <<CNF
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
CNF

openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout "$out/key.pem" -out "$out/cert.pem" -config "$out/req.cnf" >/dev/null 2>&1
openssl pkcs12 -export -inkey "$out/key.pem" -in "$out/cert.pem" \
  -out "$out/identity.p12" -passout "pass:$pass" -name "$name" >/dev/null 2>&1

base64 -i "$out/identity.p12" > "$out/identity.p12.base64"
printf '%s' "$pass" > "$out/password.txt"
printf '%s' "$name" > "$out/name.txt"
rm -f "$out/req.cnf" "$out/key.pem"
chmod 600 "$out"/*

echo "Created \"$name\" in $out"
echo
echo "Add three repository secrets. These commands copy each value to the"
echo "clipboard in turn so nothing is printed to the screen:"
echo
echo "  pbcopy < $out/identity.p12.base64   # paste as MACOS_CERTIFICATE"
echo "  pbcopy < $out/password.txt          # paste as MACOS_CERTIFICATE_PWD"
echo "  pbcopy < $out/name.txt              # paste as MACOS_SIGNING_IDENTITY"
echo
echo "at https://github.com/Oh-Damn/caddie/settings/secrets/actions"
echo
echo "Keep $out. Losing it means the next release signs as a different app and"
echo "every user has to grant permissions again."
