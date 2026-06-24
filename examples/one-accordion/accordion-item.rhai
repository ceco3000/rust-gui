// AccordionItem paint 脚本
// 绘制标题栏 (label + chevron) + 内容区背景 (展开时可见)
//
// 坐标以 widget 左上角为 (0, 0)。
// 标题栏高度由 label_font_size 动态计算（AC03）。
//
// AC02: width/height 由框架注入（来自 Taffy 布局 bounds）。
// AC07: expanded/label/content/disabled 等 prop 由框架注入为 Rhai 变量。
// AC03: heading_level prop 控制标题字体大小和语义层级。

let text_pad_left = 16.0;
let text_pad_top = 12.0;
let font_size = 16.0;

// ── heading_level prop（AC03）──
// WA renderHeadingWrapper() 对齐：heading-level 为 "1"~"6" 时渲染为对应语义标题，
// "none" 时无标题包装（纯 <button>）。
// 在此 Tier 2 实现中，语义通过 a11y 系统体现（Phase 2），
// 当前用字体大小直观区分 heading 层级。
let heading_level_val = if global::is_def_var("heading_level") { heading_level } else { "none" };
let label_font_size = switch heading_level_val {
    "1" => 28.0,
    "2" => 24.0,
    "3" => 20.0,
    "4" => 18.0,
    "5" => 16.0,
    "6" => 14.0,
    _ => font_size,  // "none" 或未定义 → 默认 16.0
};
let chevron_size = 14.0;
// AC03: header 高度根据 label 字体大小自适应，确保大标题不被裁切
let header_h = label_font_size + 24.0;
let content_h = height - header_h;

// ── prop 变量（AC07：由框架注入）──
let is_expanded = if global::is_def_var("expanded") { expanded } else { false };
let label_text = if global::is_def_var("label") { label } else { "Section Title" };
let content_text = if global::is_def_var("content") { content } else { "Accordion item content goes here." };
let is_disabled = if global::is_def_var("disabled") { disabled } else { false };
// AC04: icon-placement prop —— "end"（默认，右侧）或 "start"（左侧）
let icon_placement = if global::is_def_var("icon_placement") { icon_placement } else { "end" };

// ── 标题栏背景 ──
// outlined/filled-outlined: 浅灰背景
let header_bg = rgb(0.97, 0.97, 0.98);
fill_rect(0.0, 0.0, width, header_h, header_bg, 0.0);

// ── 标题栏底部分隔线 ──
let sep_color = rgb(0.82, 0.82, 0.85);
fill_rect(0.0, header_h - 1.0, width, 1.0, sep_color, 0.0);

// ── Label 文本 ── AC04: icon_placement 控制 label 偏移
let text_color = rgb(0.1, 0.1, 0.15);
// "start" 时 chevron 在 label 左侧，label 需向右偏移
let label_offset = if icon_placement == "start" { 28.0 } else { 16.0 };
let label_w = width - 46.0;  // 留 chevron + padding 空间
let label_text_h = label_font_size + 8.0;
draw_text(label_text, label_offset, text_pad_top, label_w, label_text_h, text_color, label_font_size);

// ── Chevron 图标 (▶ collapsed / ▼ expanded) ── AC04 + AC07
let chevron_color = rgb(0.5, 0.5, 0.55);
// AC04: icon-placement 控制 chevron 位置
// "start" → chevron 在 label 左侧; "end" → chevron 在右侧（默认）
let chevron_x = if icon_placement == "start" { 6.0 } else { width - 30.0 };
// AC03: chevron 在 header 中垂直居中
let chevron_y = (header_h - chevron_size) / 2.0;
if is_expanded {
    draw_text("\u{25BC}", chevron_x, chevron_y, 20.0, 20.0, chevron_color, chevron_size);
} else {
    draw_text("\u{25B6}", chevron_x, chevron_y, 20.0, 20.0, chevron_color, chevron_size);
}

// ── 内容区域（展开时渲染）── AC07
if is_expanded && content_h > 0.0 {
    let content_bg = rgb(0.98, 0.98, 0.98);
    fill_rect(0.0, header_h, width, content_h, content_bg, 0.0);

    // 内容文本
    let content_color = rgb(0.25, 0.25, 0.35);
    draw_text(content_text, 20.0, header_h + 12.0, width - 40.0, 22.0, content_color, 14.0);
}

paint_children();
