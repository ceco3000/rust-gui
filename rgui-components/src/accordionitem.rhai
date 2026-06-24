// AccordionItem paint 脚本
// 翻译自 WA accordion-item.ts + accordion-item.styles.ts
// — 标题栏 (label + chevron) + 内容面板 (expanded 时可见)

let pad_h = 16.0;
let pad_v = 12.0;

// --- props ---
let is_expanded = if is_def_var("expanded") { expanded } else { false };
let label_text = if is_def_var("label") { label } else { "" };
let content_text = if is_def_var("content") { content } else { "" };
let is_disabled = if is_def_var("disabled") { disabled } else { false };
let icon_placement = if is_def_var("icon_placement") { icon_placement } else { "end" };

// --- heading_level → 字体大小 ---
let heading_level_val = if is_def_var("heading_level") { heading_level } else { "3" };
let label_font_size = switch heading_level_val {
    "1" => 28.0, "2" => 24.0, "3" => 20.0,
    "4" => 18.0, "5" => 16.0, "6" => 14.0,
    _ => 16.0,
};
let header_h = label_font_size + pad_v * 2.0;

// --- alpha (disabled 视觉) ---
let alpha = if is_disabled { 0.5 } else { 1.0 };

// --- 标题栏背景 ---
let appearance = if is_def_var("appearance") { appearance } else { "outlined" };
let header_bg = switch appearance {
    "filled" => rgba(0.92, 0.92, 0.92, alpha),
    "filled-outlined" => rgba(0.97, 0.97, 0.98, alpha),
    "plain" => rgba(0.0, 0.0, 0.0, 0.0),
    _ => rgba(0.97, 0.97, 0.98, alpha),
};
fill_rect(0.0, 0.0, width, header_h, header_bg, 0.0);

// --- 项间分隔线 (plain 外观无) ---
if appearance != "plain" {
    fill_rect(0.0, header_h - 1.0, width, 1.0, rgba(0.82, 0.82, 0.85, alpha), 0.0);
}

// --- chevron 图标 ---
let chevron_text = if is_expanded { "▼" } else { "▶" };
let chevron_size = 14.0;
let chevron_c = rgba(0.5, 0.5, 0.55, alpha);
let chevron_x = if icon_placement == "start" { pad_h - 4.0 }
                else { width - pad_h - chevron_size - 2.0 };
let chevron_y = (header_h - chevron_size) / 2.0;
draw_text(chevron_text, chevron_x, chevron_y, 20.0, 20.0, chevron_c, chevron_size);

// --- label 文本 ---
let label_x = if icon_placement == "start" { pad_h + chevron_size + 6.0 } else { pad_h };
let label_w = width - label_x - pad_h - chevron_size - 6.0;
draw_text(label_text, label_x, pad_v, label_w, label_font_size + 8.0, rgba(0.1, 0.1, 0.15, alpha), label_font_size);

// --- 内容区域 (expanded 时渲染) ---
if is_expanded {
    let content_y = header_h;
    let content_h = height - content_y;
    if content_h > 0.0 {
        fill_rect(0.0, content_y, width, content_h, rgba(0.98, 0.98, 0.98, alpha), 0.0);
        if content_text != "" {
            draw_text(content_text,
                pad_h + 4.0, content_y + pad_v,
                width - (pad_h + 4.0) * 2.0, content_h - pad_v,
                rgba(0.25, 0.25, 0.35, alpha), 14.0);
        }
    }
}

paint_children();
