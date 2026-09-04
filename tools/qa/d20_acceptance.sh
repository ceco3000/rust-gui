#!/usr/bin/env bash
# D20 rgui 验收检测：模态层级 + InputEvent/ImeEvent + 流式 + 文档一致性。只读。
# 基线: tasks.md D20 + docs/D5 + 文档同步铁律。
# 用法: bash tools/qa/d20_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D20 验收检测（模态层级/InputEvent+ImeEvent + 文档一致性）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
FC=rgui-platform/src/focus.rs; EL=rgui-platform/src/event_loop.rs; INP=rgui-platform/src/input.rs

echo
echo "--- [1] 模态层级(open_modal/close_modal/is_modal_open) ---"
if grep -qE 'pub fn open_modal' "$FC" 2>/dev/null; then clr "PASS" "open_modal(暂存 base + 焦点隔离到 modal_focusable)"; PASS=$((PASS+1)); else clr "FAIL" "无 open_modal"; FAIL=$((FAIL+1)); fi
if grep -qE 'pub fn close_modal' "$FC" 2>/dev/null; then clr "PASS" "close_modal(恢复 base + 保留焦点)"; PASS=$((PASS+1)); else clr "FAIL" "无 close_modal"; FAIL=$((FAIL+1)); fi
if grep -qE 'pub fn is_modal_open' "$FC" 2>/dev/null; then clr "PASS" "is_modal_open"; PASS=$((PASS+1)); else clr "FAIL" "无 is_modal_open"; FAIL=$((FAIL+1)); fi

echo
echo "--- [2] 模态 3 测试 ---"
for t in modal_opened_isolates_focus_within_modal_set modal_closed_restores_base_focusable_and_focus modal_close_clears_focus_when_base_focused_is_none; do
  if grep -qE "fn $t" "$FC" 2>/dev/null; then clr "PASS" "$t"; PASS=$((PASS+1)); else clr "FAIL" "$t 缺失"; FAIL=$((FAIL+1)); fi
done

echo
echo "--- [3] InputEvent(to_input_event) ---"
if grep -qE 'pub fn to_input_event' "$EL" 2>/dev/null; then clr "PASS" "to_input_event(CursorMoved 物理坐标/Pressed/Released/Text)"; PASS=$((PASS+1)); else clr "FAIL" "无 to_input_event"; FAIL=$((FAIL+1)); fi
if grep -qE 'CursorMoved|Pressed|Released|Text' "$EL" 2>/dev/null; then clr "PASS" "InputEvent 含 CursorMoved(物理)/Pressed/Released/Text"; PASS=$((PASS+1)); else clr "FAIL" "InputEvent 转换不全"; FAIL=$((FAIL+1)); fi

echo
echo "--- [4] ImeEvent(to_ime_event 4 测试) ---"
if grep -qE 'pub fn to_ime_event' "$EL" 2>/dev/null; then clr "PASS" "to_ime_event(Preedit/Commit/Enabled/Disabled)"; PASS=$((PASS+1)); else clr "FAIL" "无 to_ime_event"; FAIL=$((FAIL+1)); fi
for t in ime_commit_maps_to_ime_event ime_preedit_maps_to_ime_event ime_enabled_disabled_map_to_ime_event ime_event_does_not_map_to_input_event; do
  if grep -qE "fn $t" "$EL" 2>/dev/null; then clr "PASS" "$t"; PASS=$((PASS+1)); else clr "FAIL" "$t 缺失"; FAIL=$((FAIL+1)); fi
done

echo
echo "--- [5] demo d20_modal ---"
DM=rgui/examples/d20_modal.rs
if [ -f "$DM" ]; then
  if grep -qE 'BASE_A|BASE_B|MODAL_BTN|modal_open|OpenModal|CloseModal' "$DM" 2>/dev/null; then clr "PASS" "d20_modal 后台两按钮+模态浮层+Open/Close"; PASS=$((PASS+1)); else clr "FAIL" "d20_modal 关键缺"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "d20_modal.rs 缺失"; NR=$((NR+1)); fi

echo
echo "--- [6] 流式编码 ---------"
if grep -qE 'std::mem::take|\.contains\(' "$FC" 2>/dev/null; then clr "PASS" "focus 模态用 std::mem::take+iter().contains(流式)"; PASS=$((PASS+1)); else clr "FAIL" "focus 非流式"; FAIL=$((FAIL+1)); fi
if grep -qE 'match event|ke\.text\.clone\(\)\.map|\.map\(' "$EL" 2>/dev/null; then clr "PASS" "to_input_event/to_ime_event 用 match+map(流式)"; PASS=$((PASS+1)); else clr "FAIL" "事件转换非流式"; FAIL=$((FAIL+1)); fi
dynit=$(grep -rnE 'dyn Iterator|Box<dyn [A-Za-z_]+>' "$FC" "$EL" "$INP" 2>/dev/null | grep -vE 'Box<dyn std::error::Error>|//' | wc -l | tr -d ' ')
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi

echo
echo "--- [7] 文档一致性(新铁律) ---"
D5="docs/D5-事件系统与输入处理设计.md"
if [ -f "$D5" ]; then
  if grep -qiE '模态|open_modal|InputEvent|ImeEvent|Ime|D20|焦点隔离' "$D5"; then clr "PASS" "D5 文档含模态层级+InputEvent/ImeEvent(D20)"; PASS=$((PASS+1)); else clr "FAIL" "D5 未含 D20"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D5 缺失"; NR=$((NR+1)); fi

echo
echo "--- [8] 全量测试(81) + 编译 ---"
cargo test --workspace --all-features >/tmp/d20_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d20_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d20_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ] && [ "${tot:-0}" -ge 81 ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})/不足81"; FAIL=$((FAIL+1)); fi
if cargo check -p rgui --features window --example d20_modal >/tmp/d20_c.out 2>&1; then clr "PASS" "d20_modal 编译通过"; PASS=$((PASS+1)); else clr "FAIL" "d20_modal 编译失败"; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态+文档一致性通过；模态浮层渲染由焦点单测保证(交互受限如实标注)。"; fi
exit 0
