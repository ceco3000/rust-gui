#!/usr/bin/env bash
# D13 rgui 验收检测：获焦高亮(ViewContext.focused + ▶) + 流式 + 文档一致性。只读。
# 基线: tasks.md D13 + 文档同步铁律。
# 用法: bash tools/qa/d13_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D13 验收检测（获焦高亮/流式/文档一致性）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }

echo
echo "--- [1] ViewContext.focused 字段 ---"
if grep -qE 'pub focused: *bool' rgui-core/src/context.rs 2>/dev/null; then clr "PASS" "ViewContext.focused: bool 存在"; PASS=$((PASS+1)); else clr "FAIL" "context.rs 无 focused 字段"; FAIL=$((FAIL+1)); fi

echo
echo "--- [2] components 获焦高亮(▶ 前缀) ---"
mk=$(grep -cE 'ctx\.focused|focus_marker|▶' rgui-core/src/components.rs 2>/dev/null)
echo "  components 获焦高亮相关: $mk 处"
if [ "${mk:-0}" -ge 2 ]; then clr "PASS" "Accordion/WaBadge 获焦高亮(▶前缀, via ctx.focused)"; PASS=$((PASS+1)); else clr "FAIL" "components 未见获焦高亮"; FAIL=$((FAIL+1)); fi

echo
echo "--- [3] focus_marker 测试(2 个) ---"
t=$(grep -cE 'adds_focus_marker_when_focused|focused = true' rgui-core/tests/d10_components.rs 2>/dev/null)
echo "  focus_marker 测试相关: $t 处"
if [ "${t:-0}" -ge 2 ]; then clr "PASS" "focus_marker 测试存在(accordion/badge_view_adds_focus_marker_when_focused)"; PASS=$((PASS+1)); else clr "FAIL" "focus_marker 测试缺失"; FAIL=$((FAIL+1)); fi

echo
echo "--- [4] 流式(纯值传递, 无 dyn Iterator/冗余 collect) ---"
dynit=$(grep -cE 'dyn Iterator|Box<dyn' rgui-core/src/context.rs rgui-core/src/components.rs 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')
col=$(grep -cE '\.collect\(' rgui-core/src/context.rs rgui-core/src/components.rs 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')
echo "  context/components dyn Iterator: $dynit, collect: $col"
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi
if [ "${col:-0}" = "0" ]; then clr "PASS" "无冗余 collect(纯值/format 传递)"; PASS=$((PASS+1)); else clr "NOTE" "collect 出现(${col})需人工核对"; NOTE=$((NOTE+1)); fi

echo
echo "--- [5] 文档一致性(新铁律) ---"
D5="docs/D5-事件系统与输入处理设计.md"
if [ -f "$D5" ]; then
  if grep -qiE '焦点视觉透传|focused|获焦高亮|▶|ViewContext.focused' "$D5"; then clr "PASS" "D5 文档含焦点视觉透传/获焦高亮(D13)"; PASS=$((PASS+1)); else clr "FAIL" "D5 文档未含获焦高亮(文档没同步)"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D5 缺失"; NR=$((NR+1)); fi
GF="tools/2025-09-01_rgui-greenfield-architecture.md"
if [ -f "$GF" ]; then h=$(grep -ciE 'focus|焦点|focused' "$GF"); echo "  greenfield 焦点提及: $h"; clr "NOTE" "greenfield 焦点 $h 处——人工核对与代码一致"; NOTE=$((NOTE+1)); fi

echo
echo "--- [6] 全量测试(60 期望) + 编译 ---"
cargo test --workspace --all-features >/tmp/d13_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d13_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d13_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ] && [ "${tot:-0}" -ge 60 ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})/不足60"; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态+文档一致性通过；截图确认获焦高亮(▶)见人工核对。"; fi
exit 0
