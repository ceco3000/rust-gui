// Accordion 容器 paint 脚本
// 翻译自 WA accordion.styles.ts
// — border + border-radius + background-color（4 种 appearance）
//
// AC02: width/height 由执行引擎从 LayoutEngine 注入。

let appearance = if global::is_def_var("appearance") { appearance } else { "outlined" };

// --- 背景色 ---
// WA: outlined=白底, filled=透明边框+灰色底, filled-outlined=quiet背景+quiet边框, plain=全透明
let bg = switch appearance {
    "filled" => rgba(0.92, 0.92, 0.92, 1.0),
    "filled-outlined" => rgba(0.95, 0.95, 0.97, 1.0),
    "plain" => rgba(0.0, 0.0, 0.0, 0.0),
    _ => rgba(1.0, 1.0, 1.0, 1.0),  // outlined
};

// --- 边框色 ---
let border_c = switch appearance {
    "filled" => rgba(0.0, 0.0, 0.0, 0.0),
    "filled-outlined" => rgba(0.82, 0.82, 0.85, 1.0),
    "plain" => rgba(0.0, 0.0, 0.0, 0.0),
    _ => rgba(0.82, 0.82, 0.85, 1.0),  // outlined
};

// --- 圆角（plain 无圆角） ---
let radius = if appearance == "plain" { 0.0 } else { 8.0 };

// 背景 + 圆角
fill_rect(0.0, 0.0, width, height, bg, radius);

// 四边 1px 边框（plain + filled 无边框）
if appearance != "plain" && appearance != "filled" {
    fill_rect(0.0, 0.0, width, 1.0, border_c, 0.0);             // top
    fill_rect(0.0, height - 1.0, width, 1.0, border_c, 0.0);    // bottom
    fill_rect(0.0, 0.0, 1.0, height, border_c, 0.0);            // left
    fill_rect(width - 1.0, 0.0, 1.0, height, border_c, 0.0);    // right
}

paint_children();
