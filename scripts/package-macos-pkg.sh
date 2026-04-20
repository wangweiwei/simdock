#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="Simdock"
BUNDLE_ID="com.simdock.Simdock"
VERSION="$(awk -F '"' '/^version = / { print $2; exit }' "$ROOT_DIR/Cargo.toml")"

OUT_DIR="$ROOT_DIR/target/macos"
APP_DIR="$OUT_DIR/$APP_NAME.app"
PKG_PATH="$OUT_DIR/$APP_NAME-$VERSION.pkg"
PKG_SIGN_IDENTITY="${PKG_SIGN_IDENTITY:-}"

if [[ -z "$VERSION" ]]; then
  echo "Could not read version from Cargo.toml" >&2
  exit 1
fi

"$ROOT_DIR/scripts/build-macos-app.sh"

rm -f "$PKG_PATH"

if [[ -n "$PKG_SIGN_IDENTITY" ]]; then
  echo "Creating signed PKG..."
  pkgbuild \
    --component "$APP_DIR" \
    --install-location /Applications \
    --identifier "$BUNDLE_ID" \
    --version "$VERSION" \
    --sign "$PKG_SIGN_IDENTITY" \
    "$PKG_PATH"
else
  echo "Creating unsigned PKG..."
  pkgbuild \
    --component "$APP_DIR" \
    --install-location /Applications \
    --identifier "$BUNDLE_ID" \
    --version "$VERSION" \
    "$PKG_PATH"
fi

echo "Packaged pkg: $PKG_PATH"
