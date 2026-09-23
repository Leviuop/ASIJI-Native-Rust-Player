#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if ! command -v cargo >/dev/null 2>&1; then
    echo 'Install Rust: https://rustup.rs/ (and Debian/Ubuntu packages: build-essential pkg-config libasound2-dev)' >&2
    exit 1
fi
cargo build --release --locked
mkdir -p bin
cp target/release/asiji bin/asiji
echo 'Built bin/asiji'
