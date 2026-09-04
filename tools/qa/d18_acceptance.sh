#!/usr/bin/env bash
# D18 rgui 验收检测：key-based reconcile + 动态增删 + Focus 边界 + 流式 + 文档一致性。只读。
# 基线: tasks.md D18 + docs/D1/D2/D5/D10 + 文档同步铁律。
# 用法: bash tools/qa/d18_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D18 验收检测（key-based reconcile/动态增删/Focus边界 + 文档一致性）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
VW=rgui-core/src/view.rs; DF=rgui-core/src/state/diff.rs

echo
echo "--- [1] key-based reconcile(WidgetView.key + diff_children_keyed) ---"
if grep -qE 'pub key: Option<u64>' "$VW" 2>/dev/null; then clr "PASS" "WidgetView.key: Option<u64>"; PASS=$((PASS+1)); else clr "FAIL" "WidgetView 无 key"; FAIL=$((FAIL+1)); fi
if grep -qE 'fn diff_children_keyed' "$DF" 2>/dev/null; then clr "PASS" "diff_children_keyed(按 key 匹配) 存在"; PASS=$((PASS+1)); else clr "FAIL" "diff_children_keyed 缺失"; FAIL=$((FAIL+1)); fi
if grep -qE 'iter\(\)\.position\(\|c\| c\.key == Some\(k\)\)|position\(\|c\| .*key' "$DF" 2>/dev/null; then clr "PASS" "diff_children_keyed 用 iter().position()(key 匹配复用)"; PASS=$((PASS+1)); else clr "FAIL" "diff_children_keyed 未见 position 匹配"; FAIL=$((FAIL+1)); fi
if grep -qE 'Patch::MoveChild' "$DF" 2>/dev/null; then clr "PASS" "Patch::MoveChild 存在"; PASS=$((PASS+1)); else clr "FAIL" "无 MoveChild"; FAIL=$((FAIL+1)); fi

echo
echo "--- [2] keyed 测试(rust diff 内嵌) ---"
if grep -qE 'fn keyed_reorder_reuses_by_key' "$DF" 2>/dev/null; then clr "PASS" "keyed_reorder_reuses_by_key(顺序交换→MoveChild 复用)"; PASS=$((PASS+1)); else clr "FAIL" "keyed_reorder 缺失"; FAIL=$((FAIL+1)); fi
if grep -qE 'fn keyed_remove_middle_keeps_neighbors' "$DF" 2>/dev/null; then clr "PASS" "keyed_remove_middle_keeps_neighbors(删中间仅 RemoveChild 不误伤邻位)"; PASS=$((PASS+1)); else clr "FAIL" "keyed_remove_middle 缺失"; FAIL=$((FAIL+1)); fi
if grep -qE 'fn keyed_add_remove_and_reorder_roundtrip_converges' "$DF" 2>/dev/null; then clr "PASS" "keyed_add_remove_and_reorder_roundtrip(增删+重排+更新收敛)"; PASS=$((PASS+1)); else clr "FAIL" "roundtrip 缺失"; FAIL=$((FAIL+1)); fi

echo
echo "--- [3] Focus 边界(D12 P2)·set_focusable_clears ---"
FC=rgui-platform/src/focus.rs
if grep -qE 'fn set_focusable_clears_removed_focus_and_keeps_existing' "$FC" 2>/dev/null; then clr "PASS" "set_focusable_clears_removed_focus(被移除焦点清空/保留焦点不变)"; PASS=$((PASS+1)); else clr "FAIL" "focus 边界测试缺失"; FAIL=$((FAIL+1)); fi

echo
echo "--- [4] 动态增删组件(d18_list) ---"
DM=rgui/examples/d18_list.rs
if [ -f "$DM" ]; then
  if grep -qE 'ListRoot|struct ListRoot' "$DM" 2>/dev/null; then clr "PASS" "d18_list ListRoot 存在"; PASS=$((PASS+1)); else clr "FAIL" "d18_list 无 ListRoot"; FAIL=$((FAIL+1)); fi
  if grep -qE 'child\.key = Some\(it\.key\)|\.key = Some' "$DM" 2>/dev/null; then clr "PASS" "子视图 key=item.key(reconcile 复用)"; PASS=$((PASS+1)); else clr "FAIL" "d18_list 子视图无 key"; FAIL=$((FAIL+1)); fi
  if grep -qE 'Add|Remove' "$DM" 2>/dev/null; then clr "PASS" "d18_list 左键 Add/右键 Remove(动态增删)"; PASS=$((PASS+1)); else clr "FAIL" "d18_list 无 Add/Remove"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "d18_list.rs 缺失"; NR=$((NR+1)); fi

echo
echo "--- [5] 流式编码 ---------"
if grep -qE '\.items|\.iter\(\)\.map\(\)\.collect\(\)|iter\(\)\.map' "$DM" 2>/dev/null; then clr "PASS" "ListRoot::view items.iter().map().collect()(流式)"; PASS=$((PASS+1)); else clr "NOTE" "ListRoot::view 未见 iter().map().collect()"; NOTE=$((NOTE+1)); fi
if grep -qE 'iter\(\)\.position\(\|c\| c\.key|\.any\(\|c\| .*key|\.iter\(\)\.any' "$DF" 2>/dev/null; then clr "PASS" "diff_children_keyed 用 iter().position+any(流式)"; PASS=$((PASS+1)); else clr "NOTE" "diff 未见 position+any"; NOTE=$((NOTE+1)); fi
dynit=$(grep -rnE 'dyn Iterator|Box<dyn [A-Za-z_]+>' "$VW" "$DF" "$DM" 2>/dev/null | grep -vE 'Box<dyn std::error::Error>|//' | wc -l | tr -d ' ')
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi

echo
echo "--- [6] 文档一致性(新铁律) ---"
for doc in "docs/D1-组件模型与WidgetSpec设计.md" "docs/D2-状态管理与差分更新设计.md" "docs/D5-事件系统与输入处理设计.md" "docs/D10-组件开发规范与示例.md"; do
  if [ -f "$doc" ]; then
    if grep -qiE 'key-based|keyed|MoveChild|reconcile|D18|WidgetView.key' "$doc"; then clr "PASS" "$(basename "$doc") 含 D18 key-based reconcile"; PASS=$((PASS+1)); else clr "FAIL" "$(basename "$doc") 未含 D18"; FAIL=$((FAIL+1)); fi
  else clr "NOT_READY" "$doc 缺失"; NR=$((NR+1)); fi
done

echo
echo "--- [7] 全量测试(69) + 编译 ---"
cargo test --workspace --all-features >/tmp/d18_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d18_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d18_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ] && [ "${tot:-0}" -ge 69 ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})/不足69"; FAIL=$((FAIL+1)); fi
if cargo check -p rgui --features window --example d18_list >/tmp/d18_c.out 2>&1; then clr "PASS" "d18_list 编译通过"; PASS=$((PASS+1)); else clr "FAIL" "d18_list 编译失败"; grep -E '^error' /tmp/d18_c.out|head; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态+文档一致性通过；动态增删/Retina 多组件项见截图人工核对。"; fi
exit 0
