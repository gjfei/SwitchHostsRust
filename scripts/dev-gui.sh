#!/usr/bin/env bash
# 开发模式运行 GUI
#   ./scripts/dev-gui.sh          单次运行，应用退出后终端回到 shell
#   ./scripts/dev-gui.sh --watch  监听源码变更并自动 cargo run（需 Ctrl+C 结束 watch）
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ "${1:-}" == "--watch" ]]; then
  shift
  if ! cargo watch --help &>/dev/null; then
    echo "未检测到 cargo-watch，正在安装（仅需一次）…"
    cargo install cargo-watch --locked
  fi
  # 不用 -c：Cursor 等终端可能不支持 clear，会导致 watch 直接退出
  exec cargo watch -x 'run -p egui-app' "$@"
fi

exec cargo run -p egui-app "$@"
