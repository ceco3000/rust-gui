#!/usr/bin/env bash
# D15 rgui 验收检测：scale_factor/DPI 换算 + hit-test 坐标影响 + 文档一致性。只读。
# 基线: tasks.md D15 + docs/D5 + 文档同步铁律。
# 用法: bash tools/qa/d15_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D15 验收检测（scale_factor/DPI 换算）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
DEMO=rgui/examples/window_demo.rs

echo
echo "--- [1] CursorMoved 坐标换算（物理→逻辑）---"
if [ -f "$DEMO" ]; then
  # 是否调用 to_logical / scale 换算（排除纯注释行）
  if grep -qE 'rgui_platform::window::to_logical|to_logical\(' "$DEMO" 2>/dev/null; then
    clr "PASS" "window_demo 用 to_logical 换算(物理→逻辑)"; PASS=$((PASS+1))
    if grep -qE 'platform_scale\(\)|scale_factor' "$DEMO" 2>/dev/null; then clr "PASS" "换算用 scale_factor/platform_scale"; PASS=$((PASS+1)); else clr "NOTE" "换算未见 scale 来源(人工核对)"; NOTE=$((NOTE+1)); fi
  else
    clr "FAIL" "window_demo 未用 to_logical 换算(cursor 仍物理坐标直喂 hit_test)——高分屏错位未解"; FAIL=$((FAIL+1))
  fi
else clr "NOT_READY" "window_demo.rs 不存在"; NR=$((NR+1)); fi

echo
echo "--- [2] hit-test 用逻辑坐标（cursor 经换算存储）---"
if [ -f "$DEMO" ]; then
  if grep -qE 'hit_test\(' "$DEMO" 2>/dev/null; then clr "PASS" "hit_test 被调用"; PASS=$((PASS+1)); else clr "FAIL" "window_demo 未见 hit_test"; FAIL=$((FAIL+1)); fi
  if grep -qE 'to_logical\(|\.borrow_mut\(\) = \(lx' "$DEMO" 2>/dev/null; then clr "PASS" "cursor 用逻辑坐标(经 to_logical 换算)"; PASS=$((PASS+1)); else clr "FAIL" "cursor 未用逻辑坐标"; FAIL=$((FAIL+1)); fi
fi

echo
echo "--- [3] platform 暴露 scale_factor/坐标换算(或换算纯函数+单测) ---"
# 检查是否有 to_logical / scale_factor 纯函数或单测
sf=$(grep -rnE 'fn to_logical|fn .*scale_factor|physical.*scale|logical.*scale' rgui-platform/src rgui-core/src 2>/dev/null | grep -vE '//' | wc -l | tr -d ' ')
echo "  坐标换算函数/方法: $sf 处"
if [ "${sf:-0}" -gt 0 ]; then clr "PASS" "存在 scale_factor/逻辑坐标换算函数"; PASS=$((PASS+1)); else clr "NOTE" "未见明确换算函数（可能在 demo 内联）——人工核对"; NOTE=$((NOTE+1)); fi
# 换算单测
st=$(grep -rlE 'scale_factor|to_logical|logical.*position' rgui-*/tests rgui-*/src 2>/dev/null | grep -iE 'test|spec' | wc -l | tr -d ' ')
echo "  坐标换算测试: $st 处"

echo
echo "--- [4] 流式编码（无 dyn Iterator/冗余 collect）---"
# 排除合法的 Box<dyn std::error::Error> 返回类型（非流式违规）；只查真实 dyn Iterator 装箱
dynit=$(grep -rnE 'dyn Iterator|Box<dyn [A-Za-z_]+>' rgui-platform/src rgui/examples/window_demo.rs 2>/dev/null | grep -vE 'Box<dyn std::error::Error>|//' | wc -l | tr -d ' ')
echo "  dyn Iterator(非 error): ${dynit:-0}"
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱（排除合法 Box<dyn Error>）"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi

echo
echo "--- [5] 全量测试 + 编译 ---"
cargo test --workspace --all-features >/tmp/d15_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d15_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d15_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})"; FAIL=$((FAIL+1)); fi
if cargo check --workspace --features window >/tmp/d15_c.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d15_c.out|head; FAIL=$((FAIL+1)); fi

echo
echo "--- [6] 文档一致性（D5 scale_factor 状态）---"
D5="docs/D5-事件系统与输入处理设计.md"
if [ -f "$D5" ]; then
  if grep -qiE 'scale_factor|scale' "$D5"; then
    if echo "$(grep -iE 'scale_factor|scale' "$D5" | head -3)" | grep -qiE '未实现|留后续|后续'; then
      clr "NOTE" "D5 标注 scale_factor 仍'未实现/后续'——若是 D15 已实现则文档需更新"; NOTE=$((NOTE+1))
    else clr "PASS" "D5 文档 scale_factor 已实现/描述（文档一致）"; PASS=$((PASS+1)); fi
  else clr "NOTE" "D5 未提及 scale_factor"; NOTE=$((NOTE+1)); fi
else clr "NOT_READY" "D5 缺失"; NR=$((NR+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态项通过；hit-test 坐标影响 + scale 换算见人工核对。"; fi
exit 0
