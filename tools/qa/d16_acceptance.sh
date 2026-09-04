#!/usr/bin/env bash
# D16 rgui 验收检测：StrokeRect 描边 + WidgetView.border + 获焦描边 + 流式 + 文档一致性。只读。
# 基线: tasks.md D16 + docs/D5 + 文档同步铁律。
# 用法: bash tools/qa/d16_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D16 验收检测（StrokeRect 描边/获焦描边/流式 + 文档一致性）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
SG=rgui-render/src/scene_graph.rs; VW=rgui-core/src/view.rs

echo
echo "--- [1] StrokeRect 图元 ---"
if grep -qE 'StrokeRect' "$SG" 2>/dev/null; then
  if grep -qE 'StrokeRect \{' "$SG" 2>/dev/null; then clr "PASS" "DrawCmd::StrokeRect{rect,color,width(StrokeRect)} 存在"; PASS=$((PASS+1)); else clr "FAIL" "StrokeRect 未见完整结构"; FAIL=$((FAIL+1)); fi
else clr "FAIL" "scene_graph 无 StrokeRect 图元"; FAIL=$((FAIL+1)); fi
# vello stroke 绘制
if grep -qE 'scene\.stroke|Stroke::new' rgui-render/src/vello.rs 2>/dev/null; then clr "PASS" "vello 用 stroke 绘制 StrokeRect(vello stroke 图元)"; PASS=$((PASS+1)); else clr "FAIL" "vello 未见 stroke 绘制"; FAIL=$((FAIL+1)); fi

echo
echo "--- [2] WidgetView.border + Border 类型 ---"
if grep -qE 'pub struct Border' "$VW" 2>/dev/null && grep -qE 'pub border: Option<Border>' "$VW" 2>/dev/null; then clr "PASS" "Border 类型 + WidgetView.border: Option<Border>"; PASS=$((PASS+1)); else clr "FAIL" "view.rs 缺 Border/border"; FAIL=$((FAIL+1)); fi
# from_view 识别 border → StrokeRect
if grep -qE 'if let Some\(b\) = &view\.border' "$SG" 2>/dev/null; then clr "PASS" "from_view if-let 识别 border → StrokeRect"; PASS=$((PASS+1)); else clr "FAIL" "from_view 未识别 border"; FAIL=$((FAIL+1)); fi

echo
echo "--- [3] components 获焦描边(亮黄) ---"
if grep -qE '255, 230, 80' rgui-core/src/components.rs 2>/dev/null; then
  cnt=$(grep -cE '255, 230, 80' rgui-core/src/components.rs); clr "PASS" "Accordion/WaBadge 获焦描边亮黄 rgb(255,230,80) (${cnt} 处)"; PASS=$((PASS+1)); else clr "FAIL" "components 未见获焦亮黄描边"; FAIL=$((FAIL+1)); fi
# 获焦 border is_some、未获焦 is_none 测试
if grep -qE 'gets_focus_border_when_focused' rgui-core/tests/d10_components.rs 2>/dev/null; then clr "PASS" "获焦描边测试存在(accordion/badge_view_gets_focus_border_when_focused)"; PASS=$((PASS+1)); else clr "FAIL" "获焦描边测试缺失"; FAIL=$((FAIL+1)); fi

echo
echo "--- [4] 流式编码 ---------"
# border 测试 iter().any() + from_view if-let；无 dyn Iterator/冗余 collect
if grep -qE '\.any\(\|c\| matches!.*StrokeRect' rgui-render/tests/glyph_offscreen.rs 2>/dev/null; then clr "PASS" "border 测试用 iter().any()(流式)"; PASS=$((PASS+1)); else clr "NOTE" "border 测试未见 .any(流式)人工核对"; NOTE=$((NOTE+1)); fi
if grep -qE 'if let Some\(b\) = &view\.border' "$SG" 2>/dev/null; then clr "PASS" "from_view 用 if-let(无冗余循环)"; PASS=$((PASS+1)); else clr "NOTE" "from_view border 非 if-let"; NOTE=$((NOTE+1)); fi
dynit=$(grep -rnE 'dyn Iterator|Box<dyn [A-Za-z_]+>' "$SG" "$VW" rgui-render/src/vello.rs rgui-render/tests/glyph_offscreen.rs 2>/dev/null | grep -vE 'Box<dyn std::error::Error>|//' | wc -l | tr -d ' ')
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi

echo
echo "--- [5] 文档一致性(新铁律) ---"
D5="docs/D5-事件系统与输入处理设计.md"
if [ -f "$D5" ]; then
  if grep -qiE '描边|StrokeRect|border|D16' "$D5"; then clr "PASS" "D5 文档含获焦描边边框(D16)"; PASS=$((PASS+1)); else clr "FAIL" "D5 未含描边(D16)"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D5 缺失"; NR=$((NR+1)); fi

echo
echo "--- [6] 全量测试(64) + 编译 ---"
cargo test --workspace --all-features >/tmp/d16_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d16_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d16_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ] && [ "${tot:-0}" -ge 64 ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})/不足64"; FAIL=$((FAIL+1)); fi
if cargo check --workspace --features window >/tmp/d16_c.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d16_c.out|head; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态+文档一致性通过；截图确认亮黄描边见人工核对。"; fi
exit 0
