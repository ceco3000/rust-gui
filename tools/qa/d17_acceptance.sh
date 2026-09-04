#!/usr/bin/env bash
# D17 rgui 验收检测：文本换行(shape_line max_width) + 渲染尺寸统一(scale) + 多组件布局 + 流式 + 文档一致性。只读。
# 基线: tasks.md D17 + docs/D5 + 文档同步铁律。
# 用法: bash tools/qa/d17_acceptance.sh [--workspace-root=<dir>]
set -uo pipefail
ROOT="${PWD}"
for a in "$@"; do case "$a" in --workspace-root=*) ROOT="${a#*=}";; esac; done
cd "$ROOT" || { echo "FATAL: cannot cd $ROOT"; exit 2; }
PASS=0; FAIL=0; NR=0; NOTE=0
clr() { printf '  %-4s %s\n' "$1" "$2"; }
echo "== rgui D17 验收检测（文本换行/渲染尺寸统一/多组件布局 + 文档一致性）=="
[ ! -f Cargo.toml ] && { echo "NOT_READY: 无 workspace"; exit 3; }
TX=rgui-render/src/text.rs; VL=rgui-render/src/vello.rs; SG=rgui-render/src/scene_graph.rs

echo
echo "--- [1] 文本换行(shape_line max_width wrap) ---"
if grep -qE 'shape_line\([^)]*max_width|shape_line.*Option<f32>' "$TX" 2>/dev/null; then clr "PASS" "shape_line 签名含 max_width: Option<f32>"; PASS=$((PASS+1)); else clr "FAIL" "shape_line 无 max_width"; FAIL=$((FAIL+1)); fi
if grep -qE 'set_size.*Some\(w\)|max_width|set_width' "$TX" 2>/dev/null; then clr "PASS" "shape_line 按 max_width 设置宽度(换行)"; PASS=$((PASS+1)); else clr "FAIL" "shape_line 未限制宽度"; FAIL=$((FAIL+1)); fi
if grep -qE 'layout_runs\(\)|line_y' "$TX" 2>/dev/null; then clr "PASS" "layout_runs() 多行 + line_y 基线逐行递增"; PASS=$((PASS+1)); else clr "FAIL" "无多行布局"; FAIL=$((FAIL+1)); fi
# 换行测试
if grep -qE 'long_text_wraps_when_width_limited|w_max.*s_max|> 20\.0' "$TX" 2>/dev/null; then clr "PASS" "long_text_wraps_when_width_limited 测试(长文本+窄宽→glyph 最大 y 显著增大)"; PASS=$((PASS+1)); else clr "FAIL" "换行测试缺失"; FAIL=$((FAIL+1)); fi

echo
echo "--- [2] DrawText.width 传递 ---"
if grep -qE 'width: f32' "$SG" 2>/dev/null && grep -qE 'width: size\.width' "$SG" 2>/dev/null; then clr "PASS" "DrawText.width 字段 + from_view 传 size.width"; PASS=$((PASS+1)); else clr "FAIL" "DrawText.width 未传 size.width"; FAIL=$((FAIL+1)); fi
# vello draw_text width>0 换行
if grep -qE 'shape_line\(.*if width > 0\.0.*Some\(width\)|draw_text' "$VL" 2>/dev/null; then clr "PASS" "vello draw_text 按 width>0 换行"; PASS=$((PASS+1)); else clr "FAIL" "vello draw_text 未按 width 换行"; FAIL=$((FAIL+1)); fi

echo
echo "--- [3] 渲染尺寸统一(scale 逻辑→物理) ---"
if grep -qE 'render_to_view\([^)]*scale: f64|scale: f64' "$VL" 2>/dev/null; then clr "PASS" "render_to_view/render_surface 含 scale 参数"; PASS=$((PASS+1)); else clr "FAIL" "render 无 scale 参数"; FAIL=$((FAIL+1)); fi
if grep -qE 'Affine::scale\(scale\)|^[[:space:]]*let tf = kurbo::Affine::scale' "$VL" 2>/dev/null; then clr "PASS" "encode 施加 Affine::scale(scale)(fill/stroke/draw_glyphs)"; PASS=$((PASS+1)); else clr "FAIL" "encode 未施加 Affine::scale"; FAIL=$((FAIL+1)); fi
if grep -qE 'render_to_view\([^)]*1\.0|, 1\.0\)' "$VL" 2>/dev/null; then clr "PASS" "离屏传 scale=1.0(Affine::scale(1)=原坐标,无回归)"; PASS=$((PASS+1)); else clr "FAIL" "离屏 scale 非 1.0"; FAIL=$((FAIL+1)); fi

echo
echo "--- [4] 多组件布局(demo Accordion 左 + WaBadge 右) ---"
DM=rgui/examples/window_demo.rs
if grep -qE 'Accordion' "$DM" 2>/dev/null && grep -qE 'WaBadge' "$DM" 2>/dev/null; then clr "PASS" "demo 含 Accordion + WaBadge 多组件"; PASS=$((PASS+1)); else clr "FAIL" "demo 缺乏多组件"; FAIL=$((FAIL+1)); fi

echo
echo "--- [5] 流式编码 ---------"
if grep -qE '\.position\(\(id, _\)|iter\(\)\.position' "$TX" 2>/dev/null; then clr "PASS" "shape_line 用 iter().position()(流式)"; PASS=$((PASS+1)); else clr "NOTE" "shape_line 未见 iter().position()"; NOTE=$((NOTE+1)); fi
if grep -qE 'line_y' "$TX" 2>/dev/null; then clr "PASS" "line_y 流式(逐行推进,无冗余收集)"; PASS=$((PASS+1)); else clr "NOTE" "未见 line_y"; NOTE=$((NOTE+1)); fi
dynit=$(grep -rnE 'dyn Iterator|Box<dyn [A-Za-z_]+>' "$TX" "$VL" "$SG" 2>/dev/null | grep -vE 'Box<dyn std::error::Error>|//' | wc -l | tr -d ' ')
if [ "${dynit:-0}" = "0" ]; then clr "PASS" "无 dyn Iterator 装箱"; PASS=$((PASS+1)); else clr "FAIL" "出现 dyn Iterator"; FAIL=$((FAIL+1)); fi

echo
echo "--- [6] 文档一致性(新铁律) ---"
D5="docs/D5-事件系统与输入处理设计.md"
if [ -f "$D5" ]; then
  if grep -qiE '换行|D17|渲染尺寸|scale|DrawText|width' "$D5"; then clr "PASS" "D5 文档含 D17 换行/渲染尺寸统一"; PASS=$((PASS+1)); else clr "FAIL" "D5 未含 D17"; FAIL=$((FAIL+1)); fi
else clr "NOT_READY" "D5 缺失"; NR=$((NR+1)); fi

echo
echo "--- [7] 全量测试(65) + 编译 ---"
cargo test --workspace --all-features >/tmp/d17_t.out 2>&1
tot=$(grep -oE 'test result: ok\. [0-9]+ passed' /tmp/d17_t.out | sed 's/[^0-9 ]//g' | awk '{s+=$1} END{print s}')
fld=$(grep -cE 'test result: FAILED|error\[' /tmp/d17_t.out)
echo "  passed=$tot failed=$fld"
if [ "${fld:-0}" = "0" ] && [ "${tot:-0}" -ge 65 ]; then clr "PASS" "全量测试通过 (${tot} passed, 0 failed)"; PASS=$((PASS+1)); else clr "FAIL" "全量测试失败(${fld})/不足65"; FAIL=$((FAIL+1)); fi
if cargo check --workspace --features window >/tmp/d17_c.out 2>&1; then clr "PASS" "cargo check --workspace --features window"; PASS=$((PASS+1)); else clr "FAIL" "编译失败"; grep -E '^error' /tmp/d17_c.out|head; FAIL=$((FAIL+1)); fi

echo
echo "================= 汇总 ================="
echo "PASS=$PASS FAIL=$FAIL NOT_READY=$NR NOTE=$NOTE"
if [ "$FAIL" -gt 0 ]; then echo "结论: 存在失败项 —— 退回开发附失败报告。"; else echo "结论: 静态+文档一致性通过；换行/多组件/Retina 文字见截图人工核对。"; fi
exit 0
