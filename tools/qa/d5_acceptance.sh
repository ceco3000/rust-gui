#!/usr/bin/env bash
# D5 rgui 渲染管线验收检测（feature 门控 / 防火墙 / 反向增量）——只读或可还原的静态+实测。
# 基线: greenfield §B.2/§C.2/§D + tasks.md D5。
# 用法: bash tools/qa/d5_acceptance.sh [--workspace-root=<dir>]
# 说明: 不做像素读回（GPU 依赖，见 d5_acceptance_offscreen.rs 契约锁定测试）。
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D5 渲染管线验收检测（feature/防火墙/反向增量）=="
echo "root: $ROOT"

[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }

echo
echo "--- [F] feature 门控 ---"
RC=rgui-render/Cargo.toml
if [ -f "$RC" ]; then
  echo "  ## $RC [features]"
  sed -n '/\[features\]/,/^\[/p' "$RC" | grep -vE '^\[|^$' | head
  if grep -qE 'vello-backend' "$RC"; then clr "PASS" "render 声明 vello-backend feature"; PASS=$((PASS+1)); else clr "FAIL" "render 缺少 vello-backend feature"; FAIL=$((FAIL+1)); fi
  if grep -qE '^\s*(skia|skia-safe)\s*=|dep:skia' "$RC" 2>/dev/null; then clr "FAIL" "render 残留 skia 依赖"; FAIL=$((FAIL+1)); else clr "PASS" "无 skia 依赖(仅注释提及)"; PASS=$((PASS+1)); fi
else clr "NOT_READY" "rgui-render/Cargo.toml 不存在"; NR=$((NR+1)); fi

# core default feature 检查（P2 预计偏差，仅记录）
echo "  ## rgui-core [features]"
sed -n '/\[features\]/,/^\[/p' rgui-core/Cargo.toml 2>/dev/null | grep -vE '^\[|^$' | head
if grep -qE '^\s*default\s*=\s*\[\s*\"layout\"\s*\]' rgui-core/Cargo.toml 2>/dev/null; then
  clr "NOTE" "core default=[layout] —— 与 greenfield §D default=[] 存在 P2 偏差(taffy 不违防火墙,上报)"
fi

echo
echo "--- [FW] 依赖防火墙 ---"
# core 无 GPU/render/platform
CORE_DEP=$(sed -n '/\[dependencies\]/,/^\[/{/\[dependencies\]/!p;}' rgui-core/Cargo.toml 2>/dev/null | grep -vE '^\s*#|^\s*$' | tr '\n' ' ')
echo "  core deps: ${CORE_DEP:-<空>}"
gpu_hit=0
for t in wgpu vello cosmic-text fontdb skrifa; do
  if grep -qE "^\s*${t}\s*=" rgui-core/Cargo.toml 2>/dev/null; then clr "FAIL" "core 依赖 ${t}"; gpu_hit=1; FAIL=$((FAIL+1)); fi
done
if grep -qE 'rgui-render|rgui-platform' rgui-core/Cargo.toml 2>/dev/null; then clr "FAIL" "core 依赖 render/platform"; FAIL=$((FAIL+1)); else [ "$gpu_hit" = "0" ] && clr "PASS" "core 无 GPU/render/platform 依赖"; PASS=$((PASS+1)); fi
# 源码反向引用
core_ref=$(grep -rnE 'rgui_render|rgui_platform' rgui-core/src/ 2>/dev/null | grep -vE '^\s*//' | wc -l | tr -d ' ')
[ "$core_ref" = "0" ] && { clr "PASS" "core 源码无 render/platform 引用"; PASS=$((PASS+1)); } || { clr "FAIL" "core 源码有反向引用 ($core_ref)"; FAIL=$((FAIL+1)); }
# render 只依赖 core
if grep -qE '^\s*rgui-render\s*=' rgui-platform/Cargo.toml 2>/dev/null; then clr "FAIL" "platform 依赖 render（应互不相依）"; FAIL=$((FAIL+1)); fi
if grep -qE '^\s*rgui-platform\s*=' rgui-render/Cargo.toml 2>/dev/null; then clr "FAIL" "render 依赖 platform（应互不相依）"; FAIL=$((FAIL+1)); fi

echo
echo "--- [DAG] cargo tree 无环 ---"
if cargo tree --workspace >/tmp/d5_tree.out 2>&1; then
  grep -qi cycle /tmp/d5_tree.out && { clr "FAIL" "检测到环"; grep -i cycle /tmp/d5_tree.out; FAIL=$((FAIL+1)); } || { clr "PASS" "无环"; PASS=$((PASS+1)); }
else clr "FAIL" "cargo tree 失败"; FAIL=$((FAIL+1)); fi

echo
echo "--- [INC1] 反向增量：改 render → core 不重编（待 build 产物）---"
echo "  说明：改 rgui-render/src/xxx.rs 加注释后 cargo check -p rgui-core，对比 core 主编译产物 rmeta SHA256。"
echo "  QA 口径：改 render 应不影响 core（core 不依赖 render）；若 core 重编/报错即反向泄漏。仅改注释，验证后还原。"

echo
echo "--- [回归] 前提：cargo check --workspace 不回归 D4 ---"
if cargo check --workspace >/tmp/d5_check.out 2>&1; then clr "PASS" "cargo check --workspace"; PASS=$((PASS+1)); else clr "FAIL" "cargo check 失败"; grep -E '^error' /tmp/d5_check.out | head; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR"
[ "$FAIL" -gt 0 ] && echo "结论: 存在失败项 —— 退回开发附失败报告。" || echo "结论: 静态项通过；像素读回/SceneGraph 进度见 d5_acceptance_*.rs 注入结果。"
exit 0
