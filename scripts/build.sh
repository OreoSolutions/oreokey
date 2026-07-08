#!/bin/bash
# Build OreoKey.app.
#   ./scripts/build.sh              — build nhanh cho máy hiện tại (dev)
#   ./scripts/build.sh --universal  — universal binary (arm64 + x86_64)
# Biến môi trường:
#   CODESIGN_ID  — Developer ID để ký (mặc định: ad-hoc "-")
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

UNIVERSAL=0
[[ "${1:-}" == "--universal" ]] && UNIVERSAL=1

# 1. Rust core
if [[ $UNIVERSAL == 1 ]]; then
    TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
    LIBS=()
    for t in "${TARGETS[@]}"; do
        rustup target add "$t" >/dev/null 2>&1 || true
        cargo build --release --target "$t" -p oreokey-core
        LIBS+=("target/$t/release/liboreokey_core.a")
    done
    mkdir -p target/universal
    lipo -create -output target/universal/liboreokey_core.a "${LIBS[@]}"
    LIBDIR="$ROOT/target/universal"
    ARCH_FLAGS="--arch arm64 --arch x86_64"
else
    cargo build --release -p oreokey-core
    LIBDIR="$ROOT/target/release"
    ARCH_FLAGS=""
fi

# 2. Swift app (ARCH_FLAGS cố ý không quote — có thể rỗng hoặc nhiều flag)
cd "$ROOT/app"
# SPM không theo dõi thay đổi của thư viện tĩnh Rust → xóa binary cũ
# để ép relink, tránh chạy nhầm bản cũ dù code Rust đã đổi.
find .build -type f -name OreoKey -delete 2>/dev/null || true
# shellcheck disable=SC2086
swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR" \
    -Xlinker -rpath -Xlinker @executable_path/../Frameworks

# shellcheck disable=SC2086
BIN="$(swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR" \
    -Xlinker -rpath -Xlinker @executable_path/../Frameworks --show-bin-path)/OreoKey"

# 3. Dựng bundle .app
APP="$ROOT/dist/OreoKey.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/OreoKey"
cp "$ROOT/app/Info.plist" "$APP/Contents/Info.plist"
[[ -f "$ROOT/assets/AppIcon.icns" ]] && cp "$ROOT/assets/AppIcon.icns" "$APP/Contents/Resources/"

# 3b. Nhúng Sparkle.framework vào bundle
SPARKLE_FW="$(find "$ROOT/app/.build" -type d -name 'Sparkle.framework' -path '*macos*' | head -1)"
[[ -n "$SPARKLE_FW" ]] || { echo "❌ Không tìm thấy Sparkle.framework (chạy 'swift package resolve' trong app/?)"; exit 1; }
mkdir -p "$APP/Contents/Frameworks"
rm -rf "$APP/Contents/Frameworks/Sparkle.framework"
cp -R "$SPARKLE_FW" "$APP/Contents/Frameworks/"

# 4. Ký — nested (Sparkle) trước, app sau; hardened runtime khi có Developer ID
IDENTITY="${CODESIGN_ID:--}"
FW="$APP/Contents/Frameworks/Sparkle.framework"
sign() { codesign --force --options runtime --sign "$IDENTITY" "$1" 2>/dev/null \
    || codesign --force --sign "$IDENTITY" "$1"; }
for nested in \
    "$FW/Versions/Current/XPCServices/Installer.xpc" \
    "$FW/Versions/Current/XPCServices/Downloader.xpc" \
    "$FW/Versions/Current/Autoupdate" \
    "$FW/Versions/Current/Updater.app"; do
    [[ -e "$nested" ]] && sign "$nested"
done
sign "$FW"
sign "$APP"

echo "✅ Đã build: $APP"
