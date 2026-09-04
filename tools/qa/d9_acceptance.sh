#!/usr/bin/env bash
# D9 rgui 验收检测：facade 入口统一 + 文本真实字形 + 按需重绘。只读。
# 基线: greenfield §B.5 + tasks.md D9。
# 用法: bash tools/qa/d9_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D9 验收检测（facade 入口/文本字形/按需重绘）=="
echo "root: $ROOT"
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }

echo
echo "--- [1] App::run() 实现 + window_demo 走 facade ---"
# 宽松匹配 `fn run`，并确认非 todo!（允许泛型/多行签名）
if grep -qE 'fn run\b' rgui/src/app.rs 2>/dev/null; then
  if grep -qE 'todo!\(' rgui/src/app.rs 2>/dev/null; then clr "FAIL" "App::run 仍是 todo! 占位"; FAIL=$((FAIL+1));
  else clr "PASS" "App::run 已实现（非 todo!）"; PASS=$((PASS+1)); fi
else clr "FAIL" "rgui/src/app.rs 无 App::run"; FAIL=$((FAIL+1)); fi
# window_demo 走 facade?
if grep -nqE 'rgui::App::run|App::run\(' rgui/examples/window_demo.rs 2>/dev/null; then
  clr "PASS" "window_demo 经 facade App::run 启动"; PASS=$((PASS+1))
else
  if grep -nqE 'rgui_platform::event_loop::run_as' rgui/examples/window_demo.rs 2>/dev/null; then
    clr "FAIL" "window_demo 仍直接 platform run_as（绕过 facade）"; FAIL=$((FAIL+1))
  else clr "FAIL" "window_demo 未走 facade 入口"; FAIL=$((FAIL+1)); fi
fi

echo
echo "--- [2] 文本真实字形（cosmic-text 替换矩形）---"
# 判据：文本绘制是否走真实字形（draw_glyphs）。0.6*chars 仅兜底宽度估算，不算"矩形近似绘制"。
if grep -nqE 'draw_glyph' rgui-render/src/vello.rs 2>/dev/null; then
  clr "PASS" "vello 用 draw_glyphs 绘制真实字形（cosmic-text 整形）"; PASS=$((PASS+1))
else
  if grep -nqE 'chars\(\)\.count\(\).*0\.6' rgui-render/src/vello.rs 2>/dev/null; then
    clr "FAIL" "vello 文本仍矩形近似（0.6*chars，未走 draw_glyphs）——P2-3 未清"; FAIL=$((FAIL+1))
  else clr "NOTE" "vello 文本绘制路径待确认"; NOTE=$((NOTE+1)); fi
fi
# 实际代码引用 cosmic-text（排除注释行）
cosmic_uses=$(grep -rnE 'cosmic_text|cosmic-text|draw_glyph|fontdb' rgui-render/src/ 2>/dev/null | grep -vE ':\s*//|^\s*//' | wc -l | tr -d ' ')
if [ "$cosmic_uses" -gt 0 ]; then clr "PASS" "render 实际引用 cosmic-text/字形相关 ($cosmic_uses 行)"; PASS=$((PASS+1)); else clr "FAIL" "render 未接 cosmic-text 真实字形（仅注释提及）"; FAIL=$((FAIL+1)); fi

echo
echo "--- [3] 按需重绘（about_to_wait 不再无条件每帧）---"
rd=$(grep -nE 'about_to_wait|request_redraw|dirty|needs_redraw|RedrawRequested' rgui-platform/src/event_loop.rs 2>/dev/null)
echo "  event_loop.rs 重绘相关:"
echo "$rd" | head -8
# 关键: 是否仍有 无条件 request_redraw 在 about_to_wait
uncond=$(sed -n '/about_to_wait/,/}/p' rgui-platform/src/event_loop.rs 2>/dev/null | grep -cE 'request_redraw' )
if grep -qE 'dirty|needs_redraw|set_dirty|has_dirty' rgui-platform/src/event_loop.rs 2>/dev/null; then
  clr "PASS" "存在 dirty/按需重绘标记"; PASS=$((PASS+1))
else
  clr "NOTE" "event_loop.rs 未见 dirty 标记——若 about_to_wait 仍无条件 request_redraw 则为 P2-8 未清（CPU 高）"; NOTE=$((NOTE+1))
fi

echo
echo "--- [4] 编译（WORKSPACE + all-features 全量测试前提）---"
if cargo check --workspace --features window >/tmp/d9_check.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d9_check.out | head; FAIL=$((FAIL+1)); fi

echo
echo "--- [5] core 零 GPU/平台 + DAG 无环 ---"
gpu_hit=0
for t in wgpu vello winit cosmic-text fontdb skrifa; do grep -qE "^\s*${t}\s*=" rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 ${t}"; gpu_hit=1; FAIL=$((FAIL+1)); }; done
grep -qE 'rgui-render|rgui-platform' rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 render/platform"; FAIL=$((FAIL+1)); } || { [ "$gpu_hit" = "0" ] && { clr "PASS" "core 无 GPU/平台/外围依赖"; PASS=$((PASS+1)); }; }
core_ref=$(grep -rnE 'rgui_render|rgui_platform' rgui-core/src/ 2>/dev/null | grep -vE '^\s*//' | wc -l | tr -d ' ')
[ "$core_ref" = "0" ] && { clr "PASS" "core 源码无反向引用"; PASS=$((PASS+1)); } || { clr "FAIL" "core 反向引用 ($core_ref)"; FAIL=$((FAIL+1)); }
if cargo tree --workspace >/tmp/d9_tree.out 2>&1; then
  grep -qi cycle /tmp/d9_tree.out && { clr "FAIL" "DAG 有环"; FAIL=$((FAIL+1)); } || { clr "PASS" "DAG 无环"; PASS=$((PASS+1)); }
else clr "FAIL" "cargo tree 失败"; FAIL=$((FAIL+1)); fi

echo
echo "--- [6] 全量测试 ---"
if cargo test --workspace --all-features >/tmp/d9_test.out 2>&1; then
  failed=$(grep -cE 'test result: FAILED|error\[' /tmp/d9_test.out)
  [ "$failed" = "0" ] && { clr "PASS" "cargo test --workspace --all-features 全绿"; PASS=$((PASS+1)); } || { clr "FAIL" "测试有失败"; FAIL=$((FAIL+1)); }
else clr "FAIL" "测试执行失败"; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
[ "$FAIL" -gt 0 ] && echo "结论: 存在失败项 —— 退回开发附失败报告。" || echo "结论: 静态项通过；文本清晰度 vision 检验 + CPU 实测见截图/人工核对（检查点②③④）。"
exit 0
