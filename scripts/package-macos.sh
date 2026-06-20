#!/usr/bin/env bash
# macOS 发布：使用 cargo-bundle 构建 SwitchHostsRust.app
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP_NAME="SwitchHostsRust"
WITH_DMG=false
for arg in "$@"; do
    case "$arg" in
        --dmg) WITH_DMG=true ;;
        -h|--help)
            echo "用法: $0 [--dmg]"
            echo "  --dmg  额外生成 dist/${APP_NAME}.dmg"
            exit 0
            ;;
    esac
done

if ! command -v cargo-bundle >/dev/null 2>&1; then
    echo "==> 安装 cargo-bundle"
    cargo install cargo-bundle --locked
fi

echo "==> cargo build --release -p egui-app --bin switch-hosts-rust-gui"
cargo build --release -p egui-app --bin switch-hosts-rust-gui

echo "==> cargo bundle --release -p egui-app"
# cargo-bundle 内部 build 不会传 -p，需先显式编译 GUI；跳过其重复 build
CARGO_BUNDLE_SKIP_BUILD=1 cargo bundle --release -p egui-app

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

DMG_PATH="$DIST/${APP_NAME}.dmg"

if [[ "$WITH_DMG" == true ]]; then
    echo "==> 生成 DMG"
    hdiutil create -volname "$APP_NAME" -srcfolder "$DIST_APP" -ov -format UDZO "$DMG_PATH"
fi

echo ""
echo "==> 产物:"
echo "  $DIST_APP"
echo "  $BUNDLE_APP"
if [[ "$WITH_DMG" == true ]]; then
    echo "  $DMG_PATH"
fi
echo ""
echo "运行:"
echo "  open \"$DIST_APP\""
