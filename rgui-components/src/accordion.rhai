// Accordion 容器 paint 脚本
// 翻译自 WA accordion.styles.ts — 容器级 border + border-radius + background-color
//
// AC02: width/height 由 execute_tier2_paint_scripts 从 LayoutEngine 获取 bounds 后注入。

let appearance = if global::is_def_var("appearance") { appearance } else { "outlined" };

let bg = switch appearance {
    "filled" => rgba(0.92, 0.92, 0.92, 1.0),
    "filled-outlined" => rgba(0.95, 0.95, 0.97, 1.0),
    "plain" => rgba(0.0, 0.0, 0.0, 0.0),
    _ => rgba(1.0, 1.0, 1.0, 1.0),
};

let border_c = switch appearance {
    "filled" => rgba(0.0, 0.0, 0.0, 0.0),
    "filled-outlined" => rgba(0.82, 0.82, 0.85, 1.0),
    "plain" => rgba(0.0, 0.0, 0.0, 0.0),
    _ => rgba(0.82, 0.82, 0.85, 1.0),
};

let radius = if appearance == "plain" { 0.0 } else { 8.0 };

fill_rect(0.0, 0.0, width, height, bg, radius);

if appearance != "plain" && appearance != "filled" {
    fill_rect(0.0, 0.0, width, 1.0, border_c, 0.0);
    fill_rect(0.0, height - 1.0, width, 1.0, border_c, 0.0);
    fill_rect(0.0, 0.0, 1.0, height, border_c, 0.0);
    fill_rect(width - 1.0, 0.0, 1.0, height, border_c, 0.0);
}

paint_children();
