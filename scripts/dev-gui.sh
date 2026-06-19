#!/usr/bin/env bash
# 开发模式：监听源码变更，自动 cargo run -p egui-app
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! cargo watch --help &>/dev/null; then
  echo "未检测到 cargo-watch，正在安装（仅需一次）…"
  cargo install cargo-watch --locked
fi

# 不用 -c：Cursor 等终端可能不支持 clear，会导致 watch 直接退出
exec cargo watch -x 'run -p egui-app' "$@"
