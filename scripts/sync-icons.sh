#!/usr/bin/env bash
# 从本地 SwitchHosts 同步应用/托盘图标（与 src-tauri/icons 一致）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${SWITCHHOSTS_ICONS:-/Users/jarven/Desktop/project/self/SwitchHosts/src-tauri/icons}"
DEST="$ROOT/apps/egui/icons"

if [[ ! -d "$SRC" ]]; then
    echo "error: 找不到 SwitchHosts 图标目录: $SRC" >&2
    echo "可设置环境变量 SWITCHHOSTS_ICONS 指向 src-tauri/icons" >&2
    exit 1
fi

mkdir -p "$DEST"
cp -f "$SRC"/* "$DEST/"
echo "==> 已同步 $(ls -1 "$DEST" | wc -l | tr -d ' ') 个文件到 $DEST"
