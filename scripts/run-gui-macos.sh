#!/usr/bin/env bash
# macOS：通过 .app 启动，Mission Control / Dock 才能显示正确图标（cargo run 裸二进制无 bundle 元数据）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP_NAME="SwitchHostsRust"

if ! command -v cargo-bundle >/dev/null 2>&1; then
    echo "==> 安装 cargo-bundle"
    cargo install cargo-bundle --locked
fi

echo "==> cargo build -p egui-app --bin switch-hosts-rust-gui"
cargo build -p egui-app --bin switch-hosts-rust-gui

echo "==> cargo bundle -p egui-app"
CARGO_BUNDLE_SKIP_BUILD=1 cargo bundle -p egui-app

TARGET_DIR="$(cargo metadata --format-version=1 --no-deps 2>/dev/null \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['target_directory'])")"
BUNDLE_APP="$TARGET_DIR/debug/bundle/osx/${APP_NAME}.app"

if [[ ! -d "$BUNDLE_APP" ]]; then
    echo "error: 未找到 $BUNDLE_APP" >&2
    exit 1
fi

echo "==> open $BUNDLE_APP"
open "$BUNDLE_APP"
