#!/usr/bin/env bash
# 同步阿里巴巴普惠体 3.0 Regular 到 apps/egui/assets/fonts/
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/crates/ui-assets/assets/fonts"
FONT_NAME="AlibabaPuHuiTi-3-55-Regular.ttf"
DEST_FILE="$DEST/$FONT_NAME"

mkdir -p "$DEST"

if [[ -n "${SWITCHHOSTS_FONT:-}" && -f "$SWITCHHOSTS_FONT" ]]; then
    cp -f "$SWITCHHOSTS_FONT" "$DEST_FILE"
    echo "==> 已从 SWITCHHOSTS_FONT 复制到 $DEST_FILE"
    exit 0
fi

for zip in \
    "$HOME/Downloads/AlibabaPuHuiTi-3-55-Regular.zip" \
    "$HOME/Downloads/${FONT_NAME%.ttf}.zip"; do
    if [[ -f "$zip" ]]; then
        tmp="$(mktemp -d)"
        unzip -oj "$zip" "*/$FONT_NAME" -d "$tmp" 2>/dev/null || unzip -oj "$zip" "$FONT_NAME" -d "$tmp"
        cp -f "$tmp/$FONT_NAME" "$DEST_FILE"
        rm -rf "$tmp"
        echo "==> 已从 $zip 解压到 $DEST_FILE"
        exit 0
    fi
done

for src in \
    "$HOME/Library/Fonts/$FONT_NAME" \
    "/Library/Fonts/$FONT_NAME"; do
    if [[ -f "$src" ]]; then
        cp -f "$src" "$DEST_FILE"
        echo "==> 已从 $src 复制到 $DEST_FILE"
        exit 0
    fi
done

echo "error: 找不到 $FONT_NAME" >&2
echo "请从 https://fonts.alibabagroup.com/ 下载 Regular，或设置:" >&2
echo "  SWITCHHOSTS_FONT=/path/to/$FONT_NAME $0" >&2
exit 1
