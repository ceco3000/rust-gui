//! rgui 单 Slider 示例——滑块演示。
//!
//! 本示例绘制一个范围 0-100 的 Slider，默认值 50。

use rgui::app::{App, AppConfig};
use rgui::{Color, PaintContext, PaintLayerData, Rect, Slider, SliderState, WidgetId, WidgetSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — Slider")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // Slider 坐标：窗口 300x200，居中
    let sl_bounds = Rect::new(50.0, 88.0, 200.0, 24.0);
    let sl_id = WidgetId::from_u64(1);

    app.set_scene_builder(move |_frame: u64, width: u32, height: u32| {
        let w = width as f64;
        let h = height as f64;

        let mut layers: Vec<PaintLayerData> = Vec::new();

        // --- 背景 ---
        let mut bg_ctx = PaintContext::new(Rect::new(0.0, 0.0, w, h));
        bg_ctx.fill_rect(
            Rect::new(0.0, 0.0, w, h),
            Color::new(14.0 / 255.0, 18.0 / 255.0, 28.0 / 255.0, 1.0),
            0.0,
        );
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(0),
            -1,
            Rect::new(0.0, 0.0, w, h),
            bg_ctx.into_operations(),
        ));

        // --- Slider ---
        let mut sl_ctx = PaintContext::new(sl_bounds);
        let s = SliderState::new(50.0, 0.0, 100.0);
        Slider.paint(&s, sl_bounds, &mut sl_ctx);
        layers.push(PaintLayerData::new(
            sl_id,
            0,
            sl_bounds,
            sl_ctx.into_operations(),
        ));

        layers
    });

    println!("=== rgui Slider ===\n");
    println!("窗口: 300x200 | 范围: 0-100 | 默认值: 50");

    app.run()
}
