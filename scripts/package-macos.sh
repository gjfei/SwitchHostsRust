#!/usr/bin/env bash
# macOS 发布构建：生成 release 二进制，便于手动封装 .app / .dmg
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> 构建 GUI release"
cargo build --release -p egui-app

BIN="$ROOT/target/release/switch-hosts-rust-gui"
echo "==> 产物: $BIN"
echo ""
echo "后续可手动："
echo "  1. 创建 SwitchHostsRust.app/Contents/MacOS/ 并复制二进制"
echo "  2. 添加 Info.plist 与图标"
echo "  3. 使用 hdiutil 或 create-dmg 生成 .dmg"
