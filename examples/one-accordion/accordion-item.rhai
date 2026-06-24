// AccordionItem paint 脚本
// 绘制标题栏 (label + chevron) + 内容区背景 (展开时可见)
//
// 坐标以 widget 左上角为 (0, 0)。
// 标题栏高度 44px，下方为内容区域。
//
// AC02: width/height 由框架注入（来自 Taffy 布局 bounds）。

let header_h = 44.0;
let text_pad_left = 16.0;
let text_pad_top = 12.0;
let font_size = 16.0;
let chevron_size = 14.0;
let content_h = height - header_h;

// ── 标题栏背景 ──
// outlined/filled-outlined: 浅灰背景
let header_bg = rgb(0.97, 0.97, 0.98);
fill_rect(0.0, 0.0, width, header_h, header_bg, 0.0);

// ── 标题栏底部分隔线 ──
let sep_color = rgb(0.82, 0.82, 0.85);
fill_rect(0.0, header_h - 1.0, width, 1.0, sep_color, 0.0);

// ── Label 文本（左侧对齐）──
let text_color = rgb(0.1, 0.1, 0.15);
let label_w = width - 60.0;  // 留 chevron + padding 空间
draw_text("Section Title", text_pad_left, text_pad_top, label_w, 24.0, text_color, font_size);

// ── Chevron 图标 (▶ collapsed / ▼ expanded) ──
let chevron_color = rgb(0.5, 0.5, 0.55);
let chevron_x = width - 30.0;
let chevron_y = 15.0;
// 展开时用 ▼，折叠时用 ▶
// 默认折叠:
draw_text("\u{25B6}", chevron_x, chevron_y, 20.0, 20.0, chevron_color, chevron_size);

// ── 内容区域（展开时渲染）──
// 展开状态下可见内容背景
if content_h > 0.0 {
    let content_bg = rgb(0.98, 0.98, 0.98);
    fill_rect(0.0, header_h, width, content_h, content_bg, 0.0);

    // 内容文本
    let content_color = rgb(0.25, 0.25, 0.35);
    draw_text("Accordion item content goes here.", 20.0, header_h + 12.0, width - 40.0, 22.0, content_color, 14.0);
}

paint_children();
