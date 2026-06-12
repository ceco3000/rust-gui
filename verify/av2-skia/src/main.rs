//! AV2: Skia CPU 渲染验证（无 GPU）
//!
//! skia-safe 纯 CPU 光栅化，输出 PNG，证明无 GPU 环境可正常运行。

use skia_safe::{surfaces, Color4f, Font, FontMgr, FontStyle, Paint, PaintStyle, Rect, EncodedImageFormat};
use std::fs;

#[allow(deprecated)]
fn main() {
    println!("AV2: Skia CPU rendering (no GPU)\n");

    let font_mgr = FontMgr::new();
    let tf = font_mgr
        .match_family_style("Arial", FontStyle::normal())
        .unwrap_or_else(|| font_mgr.match_family_style("DejaVu Sans", FontStyle::normal()).unwrap());

    let mut surface = surfaces::raster_n32_premul((400, 200))
        .expect("CPU raster surface 创建失败");
    let canvas = surface.canvas();
    canvas.clear(Color4f::new(0.12, 0.12, 0.12, 1.0));

    // 蓝色圆角矩形
    let mut bg = Paint::new(Color4f::new(0.16, 0.31, 0.78, 1.0), None);
    bg.set_style(PaintStyle::Fill);
    bg.set_anti_alias(true);
    canvas.draw_round_rect(Rect::from_xywh(10.0, 10.0, 380.0, 180.0), 12.0, 12.0, &bg);

    // 标题
    let font = Font::new(tf, 28.0);
    let mut white = Paint::new(Color4f::new(1.0, 1.0, 1.0, 1.0), None);
    white.set_anti_alias(true);
    canvas.draw_str("Rust GUI — Skia CPU Render", (30, 65), &font, &white);

    // 说明文字
    let small = Font::new(font_mgr.match_family_style("Arial", FontStyle::normal()).unwrap(), 16.0);
    let mut gray = Paint::new(Color4f::new(0.78, 0.78, 1.0, 1.0), None);
    gray.set_anti_alias(true);
    canvas.draw_str("No GPU required · Pure CPU software rendering", (30, 100), &small, &gray);

    let mut green = Paint::new(Color4f::new(0.39, 1.0, 0.39, 1.0), None);
    green.set_anti_alias(true);
    canvas.draw_str("✅ Verified on CPU-only machine", (30, 140), &small, &green);

    // PNG 输出
    let img = surface.image_snapshot();
    let png = img.encode_to_data(EncodedImageFormat::PNG).expect("PNG encode failed");
    fs::write("verify/av2-skia/output.png", png.as_bytes()).expect("写入 PNG 失败");

    println!("✅ output.png saved (CPU-only, no GPU)");
}
