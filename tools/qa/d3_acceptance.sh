#!/usr/bin/env bash
# D3 rgui scaffold 验收检测脚本（只读，不改动 dev 结构）
# 基线: 5-crate greenfield（用户已拍板）—— tools/2025-09-01_rgui-greenfield-architecture.md
# 用法: bash tools/qa/d3_acceptance.sh [--workspace-root=<dir>]
# 产出: 每项 PASS/FAIL/NOT_READY 汇总。不自动修改任何源码/Cargo.toml。
set -uo pipefail

ROOT="${PWD}"
for a in "$@"; do
  case "$a" in
    --workspace-root=*) ROOT="${a#*=}" ;;
  esac
done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }

PASS=0; FAIL=0; NR=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }

echo "== rgui D3 scaffold 验收检测（5-crate greenfield 基线）=="
echo "root: $ROOT"

# 前置：workspace 是否初始化
if [ ! -f Cargo.toml ]; then
  echo "NOT_READY: 仓库根无 Cargo.toml —— dev 尚未交付 workspace 骨架。"
  echo "（对照 tasks.md: 旧代码已删（ae456fe），dev 正从零 scaffold。）"
  echo "本轮不执行 PASS/FAIL 判定；待 dev 交付后重跑本脚本。"
  exit 3
fi
MEMBERS=$(awk '/^members/{flag=1} flag{print} /\]/{if(flag)exit}' Cargo.toml 2>/dev/null | tr -d '",' | grep -oE '[a-z-]+' | grep -E '^rgui' | tr '\n' ' ' | sed 's/ $//')
[ -z "$MEMBERS" ] && MEMBERS="(未解析到 members)"

echo
echo "--- [1] cargo check --workspace ---"
if cargo check --workspace >/tmp/d3_check.out 2>&1; then
  clr "PASS" "cargo check --workspace 通过"; PASS=$((PASS+1))
else
  clr "FAIL" "cargo check --workspace 失败"; grep -E '^error' /tmp/d3_check.out | head -20; FAIL=$((FAIL+1))
fi

echo
echo "--- [2] 依赖图 DAG 无环 + Cargo 依赖防火墙 ---"
if cargo tree --workspace >/tmp/d3_tree.out 2>&1; then
  if grep -qi 'cycle' /tmp/d3_tree.out; then clr "FAIL" "cargo tree 检测到环"; grep -i 'cycle' /tmp/d3_tree.out; FAIL=$((FAIL+1))
  else clr "PASS" "cargo tree 无环"; PASS=$((PASS+1)); fi
else clr "FAIL" "cargo tree --workspace 执行失败"; grep -E 'error' /tmp/d3_tree.out | head; FAIL=$((FAIL+1)); fi

# 防火墙: core 绝不含 render/platform/macros/winit/wgpu/vello/cosmic-text
CYCLE=0
if [ -f rgui-core/Cargo.toml ]; then
  for tgt in rgui-render rgui-platform rgui-macros winit wgpu vello cosmic-text cssparser; do
    if grep -qE "^${tgt}\s*=|^${tgt} =" rgui-core/Cargo.toml 2>/dev/null; then
      clr "FAIL" "防火墙违反: rgui-core 依赖 ${tgt}"; CYCLE=1; FAIL=$((FAIL+1))
    fi
  done
  [ "$CYCLE" = "0" ] && clr "PASS" "防火墙通过: core 无 GPU/平台/macros/cssparser 依赖"
else clr "NOT_READY" "rgui-core/Cargo.toml 不存在"; NR=$((NR+1)); fi
# 方向: render 与 platform 互不相依; 均只向下依赖 core
if [ -f rgui-render/Cargo.toml ] && grep -qE '^rgui-platform' rgui-render/Cargo.toml 2>/dev/null; then clr "FAIL" "rgui-render 依赖 rgui-platform（应互不相依）"; FAIL=$((FAIL+1)); fi
if [ -f rgui-platform/Cargo.toml ] && grep -qE '^rgui-render' rgui-platform/Cargo.toml 2>/dev/null; then clr "FAIL" "rgui-platform 依赖 rgui-render（应互不相依）"; FAIL=$((FAIL+1)); fi

echo
echo "--- [3] 5 个 crate 成员齐全（无 rgui-style/state/layout/components 独立 crate）---"
echo "  workspace members: ${MEMBERS}"
EXPECT="rgui-core rgui-render rgui-platform rgui-macros rgui"
cnt_ok=1
for c in $EXPECT; do
  if ! echo "$MEMBERS" | grep -qw "$c"; then clr "FAIL" "缺成员: $c"; cnt_ok=0; FAIL=$((FAIL+1)); fi
done
[ "$cnt_ok" = "1" ] && { clr "PASS" "5 成员齐全"; PASS=$((PASS+1)); }
# 禁独立外围: 若出现这些 crate 视为违规定稿
for gone in rgui-style rgui-state rgui-layout rgui-components rgui-script rgui-devtools rgui-a11y; do
  if echo "$MEMBERS" | grep -qw "$gone"; then clr "FAIL" "不应有独立 crate: $gone（已并入 core 或删除）"; FAIL=$((FAIL+1)); fi
done

echo
echo "--- [3b] rgui-macros 为独立 proc-macro ---"
if echo "$MEMBERS" | grep -qw "rgui-macros" && [ -f rgui-macros/Cargo.toml ]; then
  if grep -q 'proc-macro\s*=\s*true' rgui-macros/Cargo.toml; then clr "PASS" "rgui-macros [lib] proc-macro = true"; PASS=$((PASS+1))
  else clr "FAIL" "rgui-macros 未声明 proc-macro"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "rgui-macros 未就绪"; NR=$((NR+1)); fi

echo
echo "--- [3c] core 子模块: state/layout/components/a11y + style 占位（P1）---"
if [ -f rgui-core/src/lib.rs ]; then
  for m in "state" "layout" "components" "a11y"; do
    if grep -qE "^pub mod ${m};" rgui-core/src/lib.rs 2>/dev/null; then clr "PASS" "core::${m} 子模块存在"; else clr "FAIL" "缺 core::${m} 子模块"; FAIL=$((FAIL+1)); fi
  done
  if grep -qE "^pub mod style;" rgui-core/src/lib.rs 2>/dev/null; then clr "PASS" "core::style 占位存在（P1 实现）"; else clr "FAIL" "缺 core::style 占位模块"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "rgui-core/src/lib.rs 不存在"; NR=$((NR+1)); fi
# style 占位不得引 cssparser
if [ -f rgui-core/Cargo.toml ] && grep -qE '^cssparser' rgui-core/Cargo.toml 2>/dev/null; then clr "FAIL" "core::style 占位阶段不应引 cssparser"; FAIL=$((FAIL+1)); fi

echo
echo "--- [4] 核心 trait 契约占位签名（原材料供人工对照 greenfield §B.1）---"
for f in rgui-core/src/traits.rs rgui-core/src/lib.rs; do
  [ -f "$f" ] && { echo "  ## $f"; grep -nE 'pub trait|pub enum EventResult|fn (message_name|schema_name|schema_version|name|view|update|measure|paint|accessibility)' "$f" 2>/dev/null | head -40; }
done

echo
echo "--- [5] lint 配置克制（仅 unsafe_code deny）---"
found_deny=0; found_extreme=0
scan_lints() { [ -f "$1" ] || return; echo "  ## $1"; grep -nE 'unsafe_code|lints\.|deny|warn|unused|todo|expect|unwrap' "$1" 2>/dev/null | head -20;
  grep -qE 'unsafe_code\s*=\s*"deny"' "$1" && found_deny=1
  grep -qE 'clippy::(all|pedantic|nursery|perf|style|complexity)\s*=\s*"deny"' "$1" && found_extreme=1; }
scan_lints Cargo.toml; scan_lints lints.toml
for f in rgui-*/Cargo.toml; do scan_lints "$f"; done
[ "$found_deny" = "1" ] && { clr "PASS" "存在 unsafe_code = deny"; PASS=$((PASS+1)); } || { clr "NOT_READY" "未找到 unsafe_code = deny"; NR=$((NR+1)); }
[ "$found_extreme" = "1" ] && { clr "FAIL" "疑似旧式全 deny"; FAIL=$((FAIL+1)); }

echo
echo "--- [6] 增量编译验证（需人工执行，按 greenfield §E.3 口径）---"
echo "  口径: 【改数据/状态层 core::state → 不重编 render】(greenfield §E.3)。"
echo "  操作: 1) 向 rgui-core/src/state/xx.rs 追加一行注释"
echo "        2) cargo check -p rgui-render  (应不重编)"
echo "        3) git checkout -- rgui-core/src/state/xx.rs (还原)"
echo "  (若按'改 core 任一函数'硬验, 非架构承诺, 不纳入 PASS 判据——参考 §C 清单说明)"

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR"
[ "$FAIL" -gt 0 ] && echo "结论: 存在失败项 —— 退回开发并附失败报告。" || echo "结论: 待 dev 交付后重跑确认整体通过。"
exit 0
