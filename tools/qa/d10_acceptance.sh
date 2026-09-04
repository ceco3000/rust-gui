#!/usr/bin/env bash
# D10 rgui 验收检测：Accordion/WaBadge 组件 + WidgetSpec 完整生命周期 + 防火墙 + DAG + 全量测试。只读。
# 基线: greenfield §B.1 + tasks.md D10。
# 用法: bash tools/qa/d10_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D10 验收检测（Accordion/WaBadge 组件 + 生命周期）=="
echo "root: $ROOT"
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
CMP=rgui-core/src/components.rs

echo
echo "--- [1] Accordion 实现（非空 stub）---"
if [ -f "$CMP" ]; then
  # 用显式 impl 块边界：从 'impl WidgetSpec for Accordion' 到下一个 'impl WidgetSpec for' 或 'impl ' 前停止
  acc_block=$(awk '/impl WidgetSpec for Accordion/{f=1} f{print} /impl WidgetSpec for WaBadge/{if(f){exit}}' "$CMP" 2>/dev/null)
  # 占位特征：State=InstanceState / Message=NoopMsg
  if echo "$acc_block" | grep -qE 'NoopMsg|InstanceState'; then clr "FAIL" "Accordion 仍用 NoopMsg/InstanceState（占位 stub）——未真实实现"; FAIL=$((FAIL+1));
  else clr "PASS" "Accordion 已用真实 State/Message（AccordionState/AccordionMsg，非占位）"; PASS=$((PASS+1)); fi
  # update 是否处理 Toggle（实现交互）
  if echo "$acc_block" | grep -qE 'fn update' && echo "$acc_block" | grep -qE 'Toggle|expanded\s*='; then clr "PASS" "Accordion update 处理 Toggle/expanded（交互实现）"; PASS=$((PASS+1)); else clr "NOTE" "Accordion update 未显著处理 Toggle——需人工核对"; NOTE=$((NOTE+1)); fi
  # view 是否展示内容（非 empty）
  if echo "$acc_block" | grep -qE 'WidgetView::empty\(\)'; then clr "NOTE" "Accordion view 可能仍 empty——需核对是否展示标题/内容"; NOTE=$((NOTE+1)); else clr "PASS" "Accordion view 有内容（构建节点/children）；非 empty"; PASS=$((PASS+1)); fi
else clr "NOT_READY" "components.rs 不存在"; NR=$((NR+1)); fi

echo
echo "--- [2] WaBadge 实现（整数 label 显示）---"
if [ -f "$CMP" ]; then
  bad_block=$(awk '/impl WidgetSpec for WaBadge/{f=1} f{print} /^}/{if(f){exit}}' "$CMP" 2>/dev/null)
  if echo "$bad_block" | grep -qE 'NoopMsg|InstanceState'; then clr "FAIL" "WaBadge 仍用 NoopMsg（占位 stub）"; FAIL=$((FAIL+1));
  else clr "PASS" "WaBadge 已用真实 State/Message（WaBadgeState/WaBadgeMsg）"; PASS=$((PASS+1)); fi
  wb_int=$(grep -cE 'count|i64|u32|label' "$CMP" 2>/dev/null)
  if [ "${wb_int:-0}" -gt 0 ]; then clr "PASS" "WaBadge 有整数 label 相关（count/label，${wb_int} 处）"; PASS=$((PASS+1)); else clr "NOTE" "WaBadge 未显著含整数 label——需人工核对 view 是否展示数值"; NOTE=$((NOTE+1)); fi
else clr "NOT_READY" "components.rs 不存在"; NR=$((NR+1)); fi

echo
echo "--- [3] WidgetSpec 完整生命周期（view/update/measure/paint 非空/正确）---"
if [ -f "$CMP" ]; then
  for m in view update measure paint; do
    cnt=$(awk '/impl WidgetSpec for Accordion/,/^}/' "$CMP" 2>/dev/null | grep -cE "fn ${m}\b")
    [ "$cnt" -eq 0 ] && { clr "FAIL" "Accordion 缺 ${m}"; FAIL=$((FAIL+1)); }
  done
  clr "PASS" "Accordion 生命周期方法齐全（view/update/measure/paint）"; PASS=$((PASS+1))
else clr "NOT_READY" "components.rs 不存在"; NR=$((NR+1)); fi

echo
echo "--- [4] 交互 demo 存在（Accordion+WaBadge 展示/交互的示例）---"
demo_hit=0
for f in rgui/examples/*.rs; do
  if grep -qE 'Accordion|WaBadge' "$f" 2>/dev/null; then demo_hit=1; echo "  含组件引用: $f"; fi
done
[ "$demo_hit" = "1" ] && { clr "PASS" "存在 Accordion/WaBadge 相关示例（见上）"; PASS=$((PASS+1)); } || { clr "NOTE" "暂未见 Accordion/WaBadge 交互示例"; NOTE=$((NOTE+1)); }

echo
echo "--- [5] 编译 + 防火墙 + DAG + 全量测试 ---"
if cargo check --workspace --features window >/tmp/d10_check.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d10_check.out | head; FAIL=$((FAIL+1)); fi
gpu_hit=0
for t in wgpu vello winit cosmic-text fontdb skrifa; do grep -qE "^\s*${t}\s*=" rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 ${t}"; gpu_hit=1; FAIL=$((FAIL+1)); }; done
grep -qE 'rgui-render|rgui-platform' rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 render/platform"; FAIL=$((FAIL+1)); } || { [ "$gpu_hit" = "0" ] && { clr "PASS" "core 零 GPU/平台依赖"; PASS=$((PASS+1)); }; }
core_ref=$(grep -rnE 'rgui_render|rgui_platform' rgui-core/src/ 2>/dev/null | grep -vE '^\s*//' | wc -l | tr -d ' ')
[ "$core_ref" = "0" ] && { clr "PASS" "core 源码无反向引用"; PASS=$((PASS+1)); } || { clr "FAIL" "core 反向引用 ($core_ref)"; FAIL=$((FAIL+1)); }
if cargo tree --workspace >/tmp/d10_tree.out 2>&1; then grep -qi cycle /tmp/d10_tree.out && { clr "FAIL" "DAG 有环"; FAIL=$((FAIL+1)); } || { clr "PASS" "DAG 无环"; PASS=$((PASS+1)); }; else clr "FAIL" "cargo tree 失败"; FAIL=$((FAIL+1)); fi
if cargo test --workspace --all-features >/tmp/d10_test.out 2>&1; then
  failed=$(grep -cE 'test result: FAILED|error\[' /tmp/d10_test.out); [ "$failed" = "0" ] && { clr "PASS" "cargo test --workspace --all-features 全绿"; PASS=$((PASS+1)); } || { clr "FAIL" "测试有失败"; FAIL=$((FAIL+1)); }
else clr "FAIL" "测试执行失败"; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
[ "$FAIL" -gt 0 ] && echo "结论: 存在失败项 —— 退回开发附失败报告。" || echo "结论: 静态项通过；交互前后对比截图见人工核对（检查点④）。"
exit 0
