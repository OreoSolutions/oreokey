#!/bin/bash
# Đóng gói dist/OreoKey.app thành DMG kéo-thả.
#   ./scripts/make-dmg.sh
# Biến môi trường (tùy chọn, cần tài khoản Apple Developer):
#   CODESIGN_ID    — Developer ID Application để ký
#   NOTARY_PROFILE — profile notarytool (xcrun notarytool store-credentials)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/dist/OreoKey.app"
DMG="$ROOT/dist/OreoKey.dmg"
VERSION="$(defaults read "$APP/Contents/Info" CFBundleShortVersionString)"

[[ -d "$APP" ]] || { echo "Chưa có $APP — chạy ./scripts/build.sh --universal trước"; exit 1; }

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

rm -f "$DMG"
hdiutil create -volname "OreoKey $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null

if [[ -n "${CODESIGN_ID:-}" ]]; then
    codesign --force --sign "$CODESIGN_ID" "$DMG"
    if [[ -n "${NOTARY_PROFILE:-}" ]]; then
        xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
        xcrun stapler staple "$DMG"
    fi
else
    echo "⚠️  DMG chưa ký/notarize (đặt CODESIGN_ID + NOTARY_PROFILE khi phát hành thật)"
fi

echo "✅ Đã tạo: $DMG"
