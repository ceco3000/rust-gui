#!/usr/bin/env bash
# D7 截图验证脚本：捕获屏幕（含目标窗口）到 tools/qa/d7_screenshots/
# 用途：Qt D7 验收截图证据。macOS screencapture 全屏捕获（权限已验证生效）。
# 用法: bash tools/qa/capture_window.sh <场景名> [--wait=秒]
#   例: bash tools/qa/capture_window.sh before_click
#       bash tools/qa/capture_window.sh after_click --wait=2   # 截前等2秒让界面稳定
# 产出: tools/qa/d7_screenshots/<场景名>_<时间戳>.png
# 说明: 全程只读截屏，不改动 dev 源码。前置：屏幕录制权限已授予（已验证）。

set -uo pipefail
ROOT="${BASH_SOURCE[0]%/*}"
OUT_DIR="$ROOT/d7_screenshots"
NAME="${1:-screenshot}"
WAIT_SEC=0
for a in "$@"; do case "$a" in --wait=*) WAIT_SEC="${a#*=}";; esac; done
mkdir -p "$OUT_DIR"

if [ "$WAIT_SEC" -ne 0 ]; then echo "  [wait] ${WAIT_SEC}s ..."; sleep "$WAIT_SEC"; fi

TS=$(date +%Y%m%d_%H%M%S)
FILE="$OUT_DIR/${NAME}_${TS}.png"
echo "  [capture] $NAME -> $FILE"

# macOS screencapture 全屏：含所有可见窗口（含目标 Rust GUI 窗口）。
# 若需仅捕获某个窗口，需窗口ID（GetWindowID/osascript受辅助功能权限限制，不可靠），
# 故采用全屏捕获（已验证含窗口内容），由 vision 分析定位。
screencapture "$FILE"
if [ -s "$FILE" ]; then
  echo "  [ok] $FILE ($(stat -f%z "$FILE") bytes)"
else
  echo "  [fail] 截图失败或为空"
  exit 1
fi
