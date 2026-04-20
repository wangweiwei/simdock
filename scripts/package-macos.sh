#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="Simdock"
VERSION="$(awk -F '"' '/^version = / { print $2; exit }' "$ROOT_DIR/Cargo.toml")"

OUT_DIR="$ROOT_DIR/target/macos"
APP_DIR="$OUT_DIR/$APP_NAME.app"
DMG_ROOT="$OUT_DIR/dmg-root"
MOUNT_DIR="$OUT_DIR/mount"
ICON_SOURCE="$ROOT_DIR/assets/brand/simdock.icns"
DMG_FILE_ICON_SOURCE="$ROOT_DIR/assets/brand/png/simdock-app-icon.png"
DMG_RW="$OUT_DIR/$APP_NAME-rw.dmg"
DMG_PATH="$OUT_DIR/$APP_NAME-$VERSION.dmg"

if [[ -z "$VERSION" ]]; then
  echo "Could not read version from Cargo.toml" >&2
  exit 1
fi

"$ROOT_DIR/scripts/build-macos-app.sh"

echo "Creating DMG staging folder..."
rm -rf "$DMG_ROOT" "$MOUNT_DIR" "$DMG_RW" "$DMG_PATH"
mkdir -p "$DMG_ROOT"
cp -R "$APP_DIR" "$DMG_ROOT/"
ln -s /Applications "$DMG_ROOT/Applications"
cp "$ICON_SOURCE" "$DMG_ROOT/.VolumeIcon.icns"

echo "Creating writable DMG..."
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$DMG_ROOT" \
  -fs HFS+ \
  -format UDRW \
  -ov "$DMG_RW" >/dev/null

echo "Setting DMG volume icon..."
mkdir -p "$MOUNT_DIR"
DEVICE="$(hdiutil attach "$DMG_RW" -mountpoint "$MOUNT_DIR" -nobrowse -noverify -noautoopen | awk '/Apple_HFS/ { print $1; exit }')"
if [[ -n "$DEVICE" ]]; then
  SetFile -a C "$MOUNT_DIR"
  hdiutil detach "$DEVICE" >/dev/null
fi

echo "Compressing DMG..."
hdiutil convert "$DMG_RW" -format UDZO -imagekey zlib-level=9 -o "$DMG_PATH" >/dev/null
rm -rf "$DMG_RW" "$DMG_ROOT" "$MOUNT_DIR"

if [[ -f "$DMG_FILE_ICON_SOURCE" ]] \
  && command -v sips >/dev/null 2>&1 \
  && command -v DeRez >/dev/null 2>&1 \
  && command -v Rez >/dev/null 2>&1 \
  && command -v SetFile >/dev/null 2>&1; then
  echo "Setting DMG file icon..."
  DMG_FILE_ICON_WORK="$OUT_DIR/dmg-file-icon.png"
  DMG_FILE_ICON_RSRC="$OUT_DIR/dmg-file-icon.rsrc"
  cp "$DMG_FILE_ICON_SOURCE" "$DMG_FILE_ICON_WORK"
  sips -i "$DMG_FILE_ICON_WORK" >/dev/null
  DeRez -only icns "$DMG_FILE_ICON_WORK" > "$DMG_FILE_ICON_RSRC"
  Rez -append "$DMG_FILE_ICON_RSRC" -o "$DMG_PATH"
  SetFile -a C "$DMG_PATH"
  rm -f "$DMG_FILE_ICON_WORK" "$DMG_FILE_ICON_RSRC"
fi

echo "Packaged app: $APP_DIR"
echo "Packaged dmg: $DMG_PATH"
