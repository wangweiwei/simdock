#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo build --workspace --release

echo
echo "Binary size report:"
for bin in target/release/simdock-cli target/release/simdock-desktop; do
  if [[ -f "$bin" ]]; then
    size="$(du -h "$bin" | awk '{print $1}')"
    echo "- ${bin}: ${size}"
  fi
done
