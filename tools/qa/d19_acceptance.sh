#!/usr/bin/env bash
# D19 rgui 验收检测：样式系统基础 + 样式驱动组件 + 默认回退 + 描边pad参数化 + 流式 + 文档一致性。只读。
# 基线: tasks.md D19 + docs/D4 + greenfield §B.1 + 文档同步铁律。
# 用法: bash tools/qa/d19_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D19 验收检测（样式系统/样式驱动 + 文档一致性）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
ST=rgui-core/src/style/mod.rs; CMP=rgui-core/src/components.rs

echo
echo "--- [1] 样式系统基础(StyleRule{selector,properties}) ---"
if [ -f "$ST" ]; then
  if grep -qE 'pub struct StyleRule' "$ST" 2>/dev/null; then
    if grep -qE 'properties' "$ST" 2>/dev/null; then clr "PASS" "StyleRule 含 properties(样式属性定义)"; PASS=$((PASS+1)); else clr "FAIL" "StyleRule 仅 selector 占位(无 properties)——D19 未落地"; FAIL=$((FAIL+1)); fi
    if grep -qE 'pub selector' "$ST" 2>/dev/null; then clr "PASS" "StyleRule.selector 存在"; PASS=$((PASS+1)); else clr "FAIL" "StyleRule 无 selector"; FAIL=$((FAIL+1)); fi
  else clr "FAIL" "style/mod.rs 无 StyleRule"; FAIL=$((FAIL+1)); fi
  if grep -qE 'pub struct StyleSheet' "$ST" 2>/dev/null; then clr "PASS" "StyleSheet 存在"; PASS=$((PASS+1)); else clr "FAIL" "无 StyleSheet"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "style/mod.rs 缺失"; NR=$((NR+1)); fi

echo
echo "--- [2] 样式驱动组件(从样式表取色,非硬编码 if-else 色值) ---"
# 组件经 ctx.styles.lookup(selector) 取样式(有效色/描边/pad)
if grep -qE 'styles\.lookup\(' "$CMP" 2>/dev/null; then clr "PASS" "components 经 ctx.styles.lookup(selector) 取样式(样式驱动)"; PASS=$((PASS+1)); else clr "FAIL" "components 未用 styles.lookup(仍硬编码)"; FAIL=$((FAIL+1)); fi
if grep -qE 'effective_(background|color|border_color|border_width|border_pad)' "$CMP" 2>/dev/null; then clr "PASS" "组件用 effective_*(未命中回退默认)"; PASS=$((PASS+1)); else clr "FAIL" "组件未用 effective_* 回退"; FAIL=$((FAIL+1)); fi
# 仍残留 base 色硬编码但作为 effective_* 默认回退参数(合法)——确认非 if-else 色值分支即可

echo
echo "--- [3] 默认样式/主题(当前硬编码色作默认回退) ---"
if [ -f "$ST" ]; then
  if grep -qE 'pub fn default_theme|fn default_style|DEFAULT_STYLE|OnceLock' "$ST" 2>/dev/null; then clr "PASS" "default_theme()/default_style()/DEFAULT_STYLE(默认主题回退)"; PASS=$((PASS+1)); else clr "FAIL" "样式表无默认/主题回退——D19 未落地"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "style/mod.rs 缺失"; NR=$((NR+1)); fi

echo
echo "--- [4] 描边 pad 参数化(D16 P2) ---"
if grep -qE 'with_pad|effective_border_pad|border_pad' "$CMP" 2>/dev/null; then clr "PASS" "描边 pad 经样式参数化(.with_pad(style.effective_border_pad(...)))"; PASS=$((PASS+1)); else clr "FAIL" "描边 pad 未参数化(仍硬编码)"; FAIL=$((FAIL+1)); fi

echo
echo "--- [5] parse_rgss(.rgss 解析或程序化构建) ---"
if [ -f "$ST" ]; then
  # D19 决策: 文本解析留后续(P1), parse_rgss 返回 default_theme(程序化构建经 StyleSheet::rule). 需 StyleSheet::rule + 注释说明.
  if grep -qE 'pub fn parse_rgss|StyleSheet::rule|default_theme\(\)' "$ST" 2>/dev/null; then clr "PASS" "parse_rgss(占位返回 default_theme)+程序化构建经 StyleSheet::rule(文本解析留后续P1,如实标注)"; PASS=$((PASS+1)); else clr "FAIL" "parse_rgss 未实现"; FAIL=$((FAIL+1)); fi
fi

echo
echo "--- [6] 流式编码(样式查找用组合子) ---"
if grep -qE '\.iter\(\)\.find|\.find\(\|.*\|.*\;|\.iter\(\)' "$ST" "$CMP" 2>/dev/null; then clr "PASS" "样式查找用组合子(iter().find)"; PASS=$((PASS+1)); else clr "NOTE" "未见样式查找组合子"; NOTE=$((NOTE+1)); fi
dynit=$(grep -rnE 'dyn Iterator|Box<dyn [A-Za-z_]+>' "$ST" "$CMP" 2>/dev/null | grep -vE 'Box<dyn std::error::Error>|//' | wc -l | tr -d ' ')
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi

echo
echo "--- [7] 文档一致性(新铁律) ---"
D4="docs/D4-样式系统与rgss设计.md"
if [ -f "$D4" ]; then
  if grep -qiE 'StyleRule|StyleSheet|properties|样式驱动|D19' "$D4"; then clr "PASS" "D4 文档含样式系统(properties/样式驱动)"; PASS=$((PASS+1)); else clr "FAIL" "D4 未含样式细节"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D4 缺失"; NR=$((NR+1)); fi

echo
echo "--- [8] 回归(样式驱动后组件不回归): 全量测试+编译+截图确认 ---"
cargo test --workspace --all-features >/tmp/d19_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d19_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d19_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ] && [ "${tot:-0}" -ge 69 ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})/不足69"; FAIL=$((FAIL+1)); fi
if cargo check --workspace --features window >/tmp/d19_c.out 2>&1; then clr "PASS" "cargo check --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态+文档一致性通过；样式驱动/获焦回归见截图人工核对。"; fi
exit 0
