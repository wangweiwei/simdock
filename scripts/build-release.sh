#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo build --workspace --release

echo
echo "Release artifacts:"
find target/release -maxdepth 1 -type f -perm -111 -print
