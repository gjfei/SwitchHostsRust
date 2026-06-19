#!/usr/bin/env bash
# macOS 发布：使用 cargo-bundle 构建 SwitchHostsRust.app
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP_NAME="SwitchHostsRust"

if ! command -v cargo-bundle >/dev/null 2>&1; then
    echo "==> 安装 cargo-bundle"
    cargo install cargo-bundle --locked
fi

echo "==> cargo bundle --release -p egui-app"
cargo bundle --release -p egui-app

TARGET_DIR="$(cargo metadata --format-version=1 --no-deps 2>/dev/null \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['target_directory'])")"
BUNDLE_APP="$TARGET_DIR/release/bundle/osx/${APP_NAME}.app"
DIST="$ROOT/dist"
DIST_APP="$DIST/${APP_NAME}.app"

if [[ ! -d "$BUNDLE_APP" ]]; then
    echo "error: 未找到 $BUNDLE_APP" >&2
    exit 1
fi

mkdir -p "$DIST"
rm -rf "$DIST_APP"
cp -R "$BUNDLE_APP" "$DIST_APP"

if command -v codesign >/dev/null 2>&1; then
    echo "==> ad-hoc 签名（本地运行 Gatekeeper）"
    codesign --force --deep --sign - "$DIST_APP" >/dev/null 2>&1 || \
        echo "warning: codesign 失败，可手动运行或忽略" >&2
fi

echo ""
echo "==> 产物:"
echo "  $DIST_APP"
echo "  $BUNDLE_APP"
echo ""
echo "运行:"
echo "  open \"$DIST_APP\""
echo ""
echo "生成 DMG（可选）:"
echo "  hdiutil create -volname \"$APP_NAME\" -srcfolder \"$DIST_APP\" -ov -format UDZO \"dist/${APP_NAME}.dmg\""
