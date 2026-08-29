#!/bin/sh
export PATH="${HOME}/.cargo/bin:${PATH}"
if [ -f "${HOME}/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "${HOME}/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust from https://rustup.rs then retry." >&2
  exit 1
fi
exec "$@"
