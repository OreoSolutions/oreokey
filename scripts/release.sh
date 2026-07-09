#!/bin/bash
# Phát hành OreoKey từ máy dev: bump version → build → ký Developer ID →
# notarize → ký EdDSA (Sparkle) → cập nhật appcast → tạo GitHub Release →
# đẩy lên main. Thay cho workflow CI (ký ngay trên máy, khóa riêng không
# rời máy bạn).
#
#   ./scripts/release.sh <version>        vd: ./scripts/release.sh 0.3.0
#
# Yêu cầu:
#   - Đang ở nhánh main, cây làm việc sạch (không thay đổi chưa commit)
#   - CODESIGN_ID     — Developer ID Application (bắt buộc để ký thật)
#   - NOTARY_PROFILE  — profile notarytool (mặc định: oreokey-notary)
#   - Khóa riêng EdDSA của Sparkle nằm trong login keychain (từ generate_keys)
#   - gh đã đăng nhập (gh auth status)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-}"
NOTARY_PROFILE="${NOTARY_PROFILE:-oreokey-notary}"
DOWNLOAD_URL="https://github.com/OreoSolutions/oreokey/releases/download/v$VERSION/OreoKey.dmg"

# --- Kiểm tra tiền đề (fail nhanh trước khi build) -------------------------
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
    || { echo "❌ version phải dạng X.Y.Z (vd 0.3.0), nhận: '${VERSION}'"; exit 1; }
[[ -n "${CODESIGN_ID:-}" ]] \
    || { echo "❌ Cần đặt CODESIGN_ID (Developer ID Application) để ký bản phát hành."; exit 1; }

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[[ "$BRANCH" == "main" ]] \
    || { echo "❌ Phải phát hành từ nhánh main (đang ở '$BRANCH')."; exit 1; }
[[ -z "$(git status --porcelain)" ]] \
    || { echo "❌ Cây làm việc chưa sạch — commit/stash trước khi phát hành."; exit 1; }
if git rev-parse -q --verify "refs/tags/v$VERSION" >/dev/null; then
    echo "❌ Tag v$VERSION đã tồn tại."; exit 1
fi
command -v gh >/dev/null || { echo "❌ Thiếu gh CLI."; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "❌ gh chưa đăng nhập (chạy: gh auth login)."; exit 1; }

echo "▶ Phát hành OreoKey v$VERSION (build $(( $(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' app/Info.plist) + 1 )))"

# --- 1. Bump version -------------------------------------------------------
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" app/Info.plist
BUILD=$(( $(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' app/Info.plist) + 1 ))
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $BUILD" app/Info.plist
sed -i '' -E "s/^version = \".*\"/version = \"$VERSION\"/" core/Cargo.toml

# --- 2. Cuốn changelog → release notes ------------------------------------
NOTES="$(mktemp)"
trap 'rm -f "$NOTES"' EXIT
python3 scripts/roll-changelog.py "$VERSION" > "$NOTES"

# --- 3. Build + ký Developer ID + notarize --------------------------------
export CODESIGN_ID NOTARY_PROFILE
./scripts/build.sh --universal
./scripts/make-dmg.sh

# --- 4. Ký EdDSA cho Sparkle (khóa riêng lấy từ keychain) -----------------
SIGN_UPDATE="$(find app/.build/artifacts -name sign_update | head -1 || true)"
[[ -n "$SIGN_UPDATE" ]] \
    || { echo "❌ Không tìm thấy sign_update (chạy 'cd app && swift package resolve')."; exit 1; }
SIG_ATTRS="$("$SIGN_UPDATE" dist/OreoKey.dmg)"
echo "  EdDSA: $SIG_ATTRS"

# --- 5. Cập nhật appcast.xml ----------------------------------------------
python3 scripts/update-appcast.py "$VERSION" "$BUILD" "$DOWNLOAD_URL" "$SIG_ATTRS" --notes-file "$NOTES"

# --- 6. Commit + tag + push tag -------------------------------------------
git add app/Info.plist core/Cargo.toml CHANGELOG.md appcast.xml
git commit -m "release: $VERSION"
git tag "v$VERSION"
git push origin "v$VERSION"

# --- 7. Tạo GitHub Release (tải DMG lên TRƯỚC khi appcast lên main, tránh
#        Sparkle của người dùng gặp 404) ----------------------------------
gh release create "v$VERSION" dist/OreoKey.dmg \
    --title "OreoKey $VERSION" \
    --notes-file "$NOTES"

# --- 8. Đẩy appcast + version bump lên main -------------------------------
git push origin HEAD:main

echo "✅ Đã phát hành OreoKey v$VERSION — DMG đã ký + notarize, appcast đã cập nhật."
