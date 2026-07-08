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
# shellcheck disable=SC2086
swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR"

# shellcheck disable=SC2086
BIN="$(swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR" --show-bin-path)/OreoKey"

# 3. Dựng bundle .app
APP="$ROOT/dist/OreoKey.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/OreoKey"
cp "$ROOT/app/Info.plist" "$APP/Contents/Info.plist"

# 4. Ký
IDENTITY="${CODESIGN_ID:--}"
codesign --force --options runtime --sign "$IDENTITY" "$APP" 2>/dev/null \
    || codesign --force --sign - "$APP"

echo "✅ Đã build: $APP"
