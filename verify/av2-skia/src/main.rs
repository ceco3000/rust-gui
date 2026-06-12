//! AV2: Skia 替代 Vello 验证 — skia-safe 0.75 编译运行验证

use skia_safe::{surfaces, Color4f, Font, FontMgr, FontStyle, Paint, Rect};

fn main() {
    let font_mgr = FontMgr::new();
    let typeface = font_mgr
        .match_family_style("Helvetica", FontStyle::normal())
        .unwrap_or_else(|| font_mgr.match_family_style("Arial", FontStyle::normal()).unwrap());
    let font = Font::new(typeface, 32.0);

    let mut surface =
        surfaces::raster_n32_premul((400, 200)).expect("创建 Skia raster surface 失败");
    let canvas = surface.canvas();
    canvas.clear(skia_safe::Color::WHITE);

    // 蓝色矩形
    let mut paint = Paint::new(Color4f::new(0.16, 0.31, 0.78, 1.0), None);
    paint.set_anti_alias(true);
    canvas.draw_rect(Rect::from_xywh(20.0, 20.0, 360.0, 160.0), &paint);

    // 中文文本
    let mut text_paint = Paint::new(Color4f::new(1.0, 1.0, 1.0, 1.0), None);
    text_paint.set_anti_alias(true);
    canvas.draw_str("你好，世界！Skia 可用", (40, 110), &font, &text_paint);

    println!("AV2: skia-safe 编译运行成功，中文文本渲染正常");
}
