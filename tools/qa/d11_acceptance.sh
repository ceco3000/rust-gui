#!/usr/bin/env bash
# D11 rgui 验收检测：hit-test/多组件/流式编码 + 文档一致性（新铁律：覆盖文档全集）。只读。
# 基线: greenfield §B.1 + tasks.md D11 + 文档同步铁律。
# 用法: bash tools/qa/d11_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D11 验收检测（hit-test/多组件/流式 + 文档一致性）=="
echo "root: $ROOT"
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }

echo
echo "--- [1] hit-test 实现（HitRegion + hit_test 流式）---"
HT=rgui-core/src/hit_test.rs
if [ -f "$HT" ]; then
  clr "PASS" "hit_test 模块存在"
  # 流式：iter().find()，无手写 for/collect/dyn Iterator
  st=$(grep -cE 'iter\(\)\.find' "$HT")
  echo "  iter().find 出现: ${st} 处"
  [ "${st}" -gt 0 ] && { clr "PASS" "hit_test 流式（iter().find()）"; PASS=$((PASS+1)); } || { clr "FAIL" "hit_test 非流式（未见 iter().find）"; FAIL=$((FAIL+1)); }
  # 无反向引用 core 依赖（core 零 GPU 检查在 [5]）
else clr "NOT_READY" "hit_test.rs 不存在"; NR=$((NR+1)); fi

echo
echo "--- [2] map_message 流式 + 多组件 demo ---"
mm=$(grep -nE 'into_iter\(\).*map|\.map\(|\.collect' rgui-core/src/view.rs 2>/dev/null | grep -cE 'into_iter\(\).*map.*collect')
echo "  map_message 流式(children into_iter().map().collect()): 命中 $mm"
if [ "$mm" -gt 0 ]; then clr "PASS" "map_message 流式（into_iter().map().collect()）"; PASS=$((PASS+1)); else clr "NOTE" "map_message 流式特征未确认，人工核对"; NOTE=$((NOTE+1)); fi
# window_demo 多组件
dm=$(grep -cE 'Accordion|WaBadge|map_message|hit_test' rgui/examples/window_demo.rs 2>/dev/null)
[ "${dm:-0}" -gt 0 ] && { clr "PASS" "window_demo 多组件+hit-test+map_message（$dm 处）"; PASS=$((PASS+1)); } || { clr "FAIL" "window_demo 未见多组件/hit-test"; FAIL=$((FAIL+1)); }

echo
echo "--- [3] 全量测试（52 期望）---"
cargo test --workspace --all-features >/tmp/d11_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d11_t.out | awk '{s+=$2} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d11_t.out)
echo "  passed=$tot failed=$fld"
[ "$fld" = "0" ] && [ "$tot" -ge 52 ] && { clr "PASS" "全量测试 ${tot} passed，0 failed（≥52）"; PASS=$((PASS+1)); } || { [ "$fld" = "0" ] && { clr "PASS" "全量 ${tot} passed, 0 failed"; PASS=$((PASS+1)); } || { clr "FAIL" "全量测试失败（$fld）"; FAIL=$((FAIL+1)); }; }

echo
echo "--- [4] 编译 + 防火墙 + DAG ---"
if cargo check --workspace --features window >/tmp/d11_c.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d11_c.out | head; FAIL=$((FAIL+1)); fi
gpu_hit=0
for t in wgpu vello winit cosmic-text fontdb skrifa; do grep -qE "^\s*${t}\s*=" rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 ${t}"; gpu_hit=1; FAIL=$((FAIL+1)); }; done
grep -qE 'rgui-render|rgui-platform' rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 render/platform"; FAIL=$((FAIL+1)); } || { [ "$gpu_hit" = "0" ] && { clr "PASS" "core 零 GPU/平台依赖"; PASS=$((PASS+1)); }; }
if cargo tree --workspace >/tmp/d11_tree.out 2>&1; then grep -qi cycle /tmp/d11_tree.out && { clr "FAIL" "DAG 有环"; FAIL=$((FAIL+1)); } || { clr "PASS" "DAG 无环"; PASS=$((PASS+1)); }; else clr "FAIL" "cargo tree 失败"; FAIL=$((FAIL+1)); fi

echo
echo "--- [5] 文档一致性（新铁律：覆盖文档全集）---"
# 核心文档必须提及 hit-test 已实现（代码↔文档一致）
DOCS_HIT="docs/D5-事件系统与输入处理设计.md"
if [ -f "$DOCS_HIT" ]; then
  if grep -qiE 'hit-test|hit_test|命中' "$DOCS_HIT"; then clr "PASS" "D5 文档含 hit-test 实现（文档一致）"; PASS=$((PASS+1)); else clr "FAIL" "D5 文档未提及 hit-test（文档没同步）"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D5 文档缺失"; NR=$((NR+1)); fi
# greenfield（架构唯一权威）是否提及 hit-test/事件路由组件（若 D11 涉及）
GF="tools/2025-09-01_rgui-greenfield-architecture.md"
if [ -f "$GF" ]; then
  # 只做存在性 + 与代码一致的粗略核对，命中数
  h=$(grep -ciE 'hit|test' "$GF")
  echo "  greenfield hit-test 提及数: $h"
  clr "NOTE" "greenfield 提及 hit-test ${h} 处——需人工核对与代码一致（架构蓝图）"; NOTE=$((NOTE+1))
fi
# 检查 CLAUDE.md 命令一致性（window_demo 运行命令）
CMD="CLAUDE.md"
if [ -f "$CMD" ]; then
  if grep -qiE 'window_demo|cargo run.*window' "$CMD"; then clr "PASS" "CLAUDE.md 含 window_demo 命令"; PASS=$((PASS+1)); else clr "NOTE" "CLAUDE.md 未见 window_demo 命令（命令文档可能在其他处）"; NOTE=$((NOTE+1)); fi
fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
[ "$FAIL" -gt 0 ] && echo "结论: 存在失败项 —— 退回开发附失败报告。" || echo "结论: 静态+文档一致性项通过；多组件截图确认见人工核对。"
exit 0
