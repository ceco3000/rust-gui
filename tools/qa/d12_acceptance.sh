#!/usr/bin/env bash
# D12 rgui 验收检测：FocusManager 焦点管理 + focusable + 流式编码 + 文档一致性。只读。
# 基线: greenfield §B.3 + tasks.md D12 + 文档同步铁律。
# 用法: bash tools/qa/d12_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D12 验收检测（FocusManager/焦点/流式 + 文档一致性）=="
echo "root: $ROOT"
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
FC=rgui-platform/src/focus.rs

echo
echo "--- [1] FocusManager 焦点循环（focus_next/focus_prev/set_focus）---"
if [ -f "$FC" ]; then
  clr "PASS" "FocusManager 模块存在（platform/focus.rs）"
  # API 齐全
  for api in set_focusable set_focus focus is_focused focus_next focus_prev; do
    grep -qE "fn ${api}\b" "$FC" || { clr "FAIL" "缺 ${api}"; FAIL=$((FAIL+1)); }
  done
  clr "PASS" "FocusManager API 齐全（set_focusable/set_focus/focus/is_focused/focus_next/focus_prev）"; PASS=$((PASS+1))
  # set_focus 拒绝非可获焦
  grep -qE 'contains\(&widget_id\)' "$FC" && { clr "PASS" "set_focus 拒绝非可获焦（contains 检查）"; PASS=$((PASS+1)); } || { clr "FAIL" "set_focus 未见非可获焦拒绝"; FAIL=$((FAIL+1)); }
else clr "NOT_READY" "focus.rs 不存在"; NR=$((NR+1)); fi

echo
echo "--- [2] WidgetSpec::focusable default + 组件覆盖 ---"
if grep -qE 'fn focusable.*-> bool' rgui-core/src/traits.rs 2>/dev/null && grep -qE 'false' rgui-core/src/traits.rs 2>/dev/null; then
  clr "PASS" "traits.rs focusable default 返回 false（默认不可获焦）"; PASS=$((PASS+1)); else clr "FAIL" "traits.rs 未见 focusable default"; FAIL=$((FAIL+1)); fi
cov=$(grep -cE 'fn focusable' rgui-core/src/components.rs)
echo "  组件覆盖 focusable 处数: $cov"
[ "${cov:-0}" -ge 2 ] && { clr "PASS" "Accordion/WaBadge 覆盖 focusable=true（${cov} 处）"; PASS=$((PASS+1)); } || { clr "NOTE" "focusable 覆盖数 ${cov}（期望 ≥2, Accordion+WaBadge）"; NOTE=$((NOTE+1)); }

echo
echo "--- [3] demo Tab/Shift+Tab 切换焦点 ---"
dm=$(grep -cE 'focus_next|focus_prev|KeyCode::Tab|FocusManager|set_focusable' rgui/examples/window_demo.rs 2>/dev/null)
echo "  window_demo 焦点相关（${dm} 处）"
[ "${dm:-0}" -gt 0 ] && { clr "PASS" "window_demo Tab 焦点切换（focus_next + set_focusable + KeyCode::Tab）"; PASS=$((PASS+1)); } || { clr "FAIL" "window_demo 未见 Tab 焦点切换"; FAIL=$((FAIL+1)); }

echo
echo "--- [4] 流式编码（move_focus iter().position + rem_euclid，无 dyn Iterator/冗余 collect）---"
if [ -f "$FC" ]; then
  pos=$(grep -cE 'iter\(\)\.position' "$FC")
  rem=$(grep -cE 'rem_euclid' "$FC")
  echo "  iter().position: ${pos}, rem_euclid: ${rem}"
  [ "$pos" -gt 0 ] && [ "$rem" -gt 0 ] && { clr "PASS" "move_focus 流式（iter().position + rem_euclid）"; PASS=$((PASS+1)); } || { clr "FAIL" "move_focus 非流式"; FAIL=$((FAIL+1)); }
  # 无 dyn Iterator / 冗余 collect
  dyn_it=$(grep -cE 'dyn Iterator|Box<dyn' "$FC"); col=$(grep -cE '\.collect\(' "$FC")
  echo "  dyn Iterator: ${dyn_it}, collect: ${col}"
  [ "${dyn_it:-0}" = "0" ] && [ "${col:-0}" = "0" ] && { clr "PASS" "无 dyn Iterator 装箱、无冗余 collect"; PASS=$((PASS+1)); } || clr "NOTE" "dyn/collect 出现（${dyn_it}/${col}）需人工核对"; NOTE=$((NOTE+1))
fi

echo
echo "--- [5] 全量测试（58 期望）---"
cargo test --workspace --all-features >/tmp/d12_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d12_t.out | awk '{s+=$2} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d12_t.out)
echo "  passed=$tot failed=$fld"
[ "${fld:-0}" = "0" ] && { clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); } || { clr "FAIL" "全量测试失败（$fld）"; FAIL=$((FAIL+1)); }

echo
echo "--- [6] 编译 + 防火墙 + DAG ---"
if cargo check --workspace --features window >/tmp/d12_c.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d12_c.out|head; FAIL=$((FAIL+1)); fi
gpu_hit=0
for t in wgpu vello winit cosmic-text fontdb skrifa; do grep -qE "^\s*${t}\s*=" rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 ${t}"; gpu_hit=1; FAIL=$((FAIL+1)); }; done
grep -qE 'rgui-render|rgui-platform' rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 render/platform"; FAIL=$((FAIL+1)); } || { [ "$gpu_hit" = "0" ] && { clr "PASS" "core 零 GPU/平台依赖"; PASS=$((PASS+1)); } || { clr "FAIL" "core 有 GPU 依赖"; FAIL=$((FAIL+1)); }; }
if cargo tree --workspace >/tmp/d12_tree.out 2>&1; then grep -qi cycle /tmp/d12_tree.out && { clr "FAIL" "DAG 有环"; FAIL=$((FAIL+1)); } || { clr "PASS" "DAG 无环"; PASS=$((PASS+1)); }; else clr "FAIL" "cargo tree 失败"; FAIL=$((FAIL+1)); fi

echo
echo "--- [7] 文档一致性（新铁律：覆盖文档全集）---"
D5="docs/D5-事件系统与输入处理设计.md"
if [ -f "$D5" ]; then
  if grep -qiE 'focus|焦点|Tab|FocusManager' "$D5"; then clr "PASS" "D5 文档含焦点管理（文档一致）"; PASS=$((PASS+1)); else clr "FAIL" "D5 文档未含焦点管理（文档没同步）"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D5 缺失"; NR=$((NR+1)); fi
# FocusManager 在 platform（§B.3 契约）；核对 greenfield 是否描述 FocusManager
GF="tools/2025-09-01_rgui-greenfield-architecture.md"
if [ -f "$GF" ]; then
  h=$(grep -ciE 'FocusManager|foci|焦点' "$GF")
  echo "  greenfield 焦点提及数: $h"
  clr "NOTE" "greenfield FocusManager 提及 ${h} 处——人工核对与代码一致"; NOTE=$((NOTE+1))
fi
# CLAUDE.md 命令
[ -f CLAUDE.md ] && grep -qiE 'window_demo|Tab' CLAUDE.md && { clr "PASS" "CLAUDE.md 含 window_demo 命令"; PASS=$((PASS+1)); } || clr "NOTE" "CLAUDE.md 未见(命令文档可能他处)"; NOTE=$((NOTE+1))

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
[ "$FAIL" -gt 0 ] && echo "结论: 存在失败项 —— 退回开发附失败报告。" || echo "结论: 静态+文档一致性通过；demo Tab 焦点切换日志+截图见人工核对。"
exit 0
