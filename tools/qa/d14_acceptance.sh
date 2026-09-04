#!/usr/bin/env bash
# D14 rgui 验收检测：获焦背景高亮(替代▶) + 流式 + 文档一致性。只读。
# 基线: tasks.md D14 + 文档同步铁律。
# 用法: bash tools/qa/d14_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D14 验收检测（获焦背景高亮/流式/文档一致性）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }

echo
echo "--- [1] 获焦背景高亮(Accordion/WaBadge 提亮, 无▶前缀) ---"
# Accordion 获焦亮色 + 普通色
if grep -qE '140, 185, 255' rgui-core/src/components.rs 2>/dev/null && grep -qE '90, 130, 220' rgui-core/src/components.rs 2>/dev/null; then
  clr "PASS" "Accordion 获焦背景 rgb(140,185,255) vs 普通 rgb(90,130,220)"; PASS=$((PASS+1)); else clr "FAIL" "Accordion 未见获焦/普通背景色"; FAIL=$((FAIL+1)); fi
if grep -qE '170, 210, 255' rgui-core/src/components.rs 2>/dev/null && grep -qE '120, 160, 210' rgui-core/src/components.rs 2>/dev/null; then
  clr "PASS" "WaBadge 获焦背景 rgb(170,210,255) vs 普通 rgb(120,160,210)"; PASS=$((PASS+1)); else clr "FAIL" "WaBadge 未见获焦/普通背景色"; FAIL=$((FAIL+1)); fi
# ▶ 前缀应移除(D14 替代背景高亮)
if grep -qE '▶' rgui-core/src/components.rs 2>/dev/null; then clr "NOTE" "components 仍含 ▶(D14 应移除,用背景高亮)——人工核对是否残留"; NOTE=$((NOTE+1)); else clr "PASS" "无 ▶ 前缀(背景高亮替代)"; PASS=$((PASS+1)); fi

echo
echo "--- [2] 背景高亮测试(2 个) ---"
if grep -qE 'background_highlights_when_focused' rgui-core/tests/d10_components.rs 2>/dev/null; then
  t=$(grep -cE 'background_highlights_when_focused' rgui-core/tests/d10_components.rs); clr "PASS" "contains_color 背景高亮测试存在(${t} 个, accordion/badge_view_background_highlights_when_focused)"; PASS=$((PASS+1)); else clr "FAIL" "背景高亮测试缺失"; FAIL=$((FAIL+1)); fi

echo
echo "--- [3] 流式(Color 条件表达式, 无 dyn Iterator/冗余 collect) ---"
if grep -qE 'if ctx\.focused' rgui-core/src/components.rs 2>/dev/null; then clr "PASS" "获焦颜色用条件表达式(if ctx.focused)"; PASS=$((PASS+1)); else clr "FAIL" "未见条件表达式"; FAIL=$((FAIL+1)); fi
dynit=$(grep -cE 'dyn Iterator|Box<dyn' rgui-core/src/components.rs 2>/dev/null); col=$(grep -cE '\.collect\(' rgui-core/src/components.rs 2>/dev/null)
echo "  components dyn Iterator: ${dynit:-0}, collect: ${col:-0}"
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi
if [ "${col:-0}" = "0" ]; then clr "PASS" "无冗余 collect(条件表达式)"; PASS=$((PASS+1)); else clr "NOTE" "collect 出现(${col})需人工核对"; NOTE=$((NOTE+1)); fi

echo
echo "--- [4] 文档一致性(新铁律) ---"
D5="docs/D5-事件系统与输入处理设计.md"
if [ -f "$D5" ]; then
  if grep -qiE '背景.*高亮|背景变亮|D14|获焦.*背景|rgb\(140|rgb\(170' "$D5"; then clr "PASS" "D5 文档含获焦背景高亮(D14, 升级▶为背景变亮)"; PASS=$((PASS+1)); else clr "FAIL" "D5 文档未含背景高亮(D14)"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D5 缺失"; NR=$((NR+1)); fi

echo
echo "--- [5] 全量测试(60) + 编译 ---"
cargo test --workspace --all-features >/tmp/d14_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d14_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d14_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ] && [ "${tot:-0}" -ge 60 ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})/不足60"; FAIL=$((FAIL+1)); fi
if cargo check --workspace --features window >/tmp/d14_c.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d14_c.out|head; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态+文档一致性通过；截图确认获焦背景高亮 vs 未获焦见人工核对。"; fi
exit 0
