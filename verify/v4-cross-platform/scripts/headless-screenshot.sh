#!/bin/bash
# V4 验证：macOS 离屏渲染截图脚本
# 用法: ./headless-screenshot.sh [输出文件名] [运行秒数]
# 默认: verify_macos.png 2

set -euo pipefail

OUTPUT="${1:-verify_macos.png}"
DURATION="${2:-2}"

CARGO_MANIFEST="$(dirname "$0")/../../v1-vello-cosmic/Cargo.toml"
CARGO_MANIFEST="$(cd "$(dirname "$CARGO_MANIFEST")" && pwd)/Cargo.toml"

echo "=== V4 离屏渲染验证（macOS）==="
echo "输出文件: $OUTPUT"
echo "运行时长: ${DURATION}s"
echo ""

# 编译
echo "--- 编译 ---"
cargo build --release --manifest-path "$CARGO_MANIFEST"
echo "编译完成"
echo ""

# 离屏运行（macOS 可通过创建虚拟显示或使用软件渲染）
echo "--- 运行 ---"
# macOS 上需要 GPU 或 Metal 软件光栅化器
# 若 CI 中无 GPU，需设置 WGPU_BACKEND=metal
cargo run --release --manifest-path "$CARGO_MANIFEST" -- \
    --headless --screenshot "$OUTPUT" --duration "$DURATION" \
    2>&1 | head -50

echo ""
if [ -f "$OUTPUT" ]; then
    SIZE=$(stat -f%z "$OUTPUT" 2>/dev/null || echo "0")
    echo "✅ 截图已生成: $OUTPUT ($SIZE bytes)"
else
    echo "❌ 截图未生成"
    exit 1
fi
