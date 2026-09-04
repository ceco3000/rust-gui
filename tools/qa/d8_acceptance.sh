#!/usr/bin/env bash
# D8 rgui 窗口逻辑收敛验收检测：winit/wgpu 引用收敛 + 编译 + 防火墙 + DAG。只读。
# 基线: greenfield §B.3/§C.3 + tasks.md D8。
# 用法: bash tools/qa/d8_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D8 窗口逻辑收敛验收检测=="
echo "root: $ROOT"
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }

# 允许含 winit/wgpu 的 crate（仅内部允许）
ALLOW="rgui-render rgui-platform"

echo
echo "--- [1] winit/wgpu 引用收敛（window_demo/facade 应 0 直接引用）---"
# 扫描所有含 winit:: / wgpu:: 的 .rs，按 crate 归类
echo "  含 winit::/wgpu:: 引用的文件:"
RL=$(grep -rlE 'winit::|wgpu::' --include='*.rs' . 2>/dev/null | grep -v target)
if [ -z "$RL" ]; then echo "    (无)"; clr "PASS" "全仓无直接 winit::/wgpu:: 引用"; PASS=$((PASS+1))
else
  has_violation=0
  echo "$RL" | while read -r f; do
    crate=$(echo "$f" | awk -F/ '{print $2}')
    allowed=0
    for a in $ALLOW; do [ "$crate" = "$a" ] && allowed=1; done
    if [ "$allowed" = "1" ]; then
      echo "    [OK 允许] $crate: $f"
    else
      echo "    [VIOLATION] $crate: $f"
      has_violation=1
    fi
  done
  # 关键：window_demo 和 facade(rgui/src) 必须 0
  # window_demo 和 facade(rgui/src) 必须 0（排除注释行）
  demo=$(grep -E 'winit::|wgpu::' rgui/examples/window_demo.rs 2>/dev/null | grep -vE '^\s*//' | wc -l | tr -d ' ')
  face=$(grep -rnE 'winit::|wgpu::' rgui/src/ 2>/dev/null | grep -vE ':\s*//' | wc -l | tr -d ' ')
  echo "  window_demo 直接引用: $demo 处;  facade rgui/src: $face 处"
  if [ "${demo:-0}" -gt 0 ]; then clr "FAIL" "window_demo 仍直接 winit::/wgpu::（${demo}处）——D8 未收敛"; FAIL=$((FAIL+1)); else clr "PASS" "window_demo 无直接 winit::/wgpu::"; PASS=$((PASS+1)); fi
  if [ "${face:-0}" -gt 0 ]; then clr "FAIL" "facade rgui/src 仍直接 winit::/wgpu::（${face}处）"; FAIL=$((FAIL+1)); else clr "PASS" "facade 无直接 winit::/wgpu::"; PASS=$((PASS+1)); fi
fi

echo
echo "--- [2] platform 公共 API（window + event_loop 封装）---"
echo "  ## window.rs pub 项:"
grep -nE 'pub fn|pub struct|pub type|pub enum' rgui-platform/src/window.rs 2>/dev/null | grep -vE '^\s*//'
echo "  ## event_loop.rs pub 项:"
grep -nE 'pub fn|pub struct|pub type|pub enum' rgui-platform/src/event_loop.rs 2>/dev/null | grep -vE '^\s*//'
has_app=0; grep -rqE 'ApplicationHandler|fn run_app|fn run\b|window_event' rgui-platform/src/ 2>/dev/null && has_app=1
if [ "$has_app" = "1" ]; then clr "PASS" "platform 含事件循环驱动(ApplicationHandler/run/run_app)"; PASS=$((PASS+1)); else clr "NOTE" "platform 可能仍占位（ApplicationHandler 组装在 facade）——需人工核对是否提供完整 window+event_loop 公共 API"; NOTE=$((NOTE+1)); fi

echo
echo "--- [3] 编译（含 window feature）---"
if cargo check --workspace --features window >/tmp/d8_check.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d8_check.out | head; FAIL=$((FAIL+1)); fi

echo
echo "--- [4] core 防火墙 ---"
gpu_hit=0
for t in wgpu vello winit cosmic-text fontdb skrifa; do
  grep -qE "^\s*${t}\s*=" rgui-core/Cargo.toml 2>/dev/null && { clr "FAIL" "core 依赖 ${t}"; gpu_hit=1; FAIL=$((FAIL+1)); }
done
if grep -qE 'rgui-render|rgui-platform' rgui-core/Cargo.toml 2>/dev/null; then clr "FAIL" "core 依赖 render/platform"; FAIL=$((FAIL+1)); else [ "$gpu_hit" = "0" ] && { clr "PASS" "core 无 GPU/平台依赖"; PASS=$((PASS+1)); }; fi
core_ref=$(grep -rnE 'rgui_render|rgui_platform' rgui-core/src/ 2>/dev/null | grep -vE '^\s*//' | wc -l | tr -d ' ')
[ "$core_ref" = "0" ] && { clr "PASS" "core 源码无 render/platform 引用"; PASS=$((PASS+1)); } || { clr "FAIL" "core 反向引用 ($core_ref)"; FAIL=$((FAIL+1)); }

echo
echo "--- [5] DAG 无环 ---"
if cargo tree --workspace >/tmp/d8_tree.out 2>&1; then
  grep -qi cycle /tmp/d8_tree.out && { clr "FAIL" "检测到环"; grep -i cycle /tmp/d8_tree.out; FAIL=$((FAIL+1)); } || { clr "PASS" "无环"; PASS=$((PASS+1)); }
else clr "FAIL" "cargo tree 失败"; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
[ "$FAIL" -gt 0 ] && echo "结论: 存在失败项 —— 退回开发附失败报告。" || echo "结论: 静态项通过；platform API 完整性 + 截图回归见人工核对（检查点②③）。"
exit 0
