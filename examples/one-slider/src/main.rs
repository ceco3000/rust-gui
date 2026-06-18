//! rgui 单 Slider 示例——滑块演示。
//!
//! 本示例绘制一个范围 0-100 的 Slider，默认值 50。

use rgui::app::{App, AppConfig};
use rgui::{
    PaintContext, PaintLayerData, Rect, Slider, SliderState, WidgetId, WidgetSpec,
    build_scene_from_paint_data,
};

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

    app.set_view_scene_builder(
        move |frame: u64, _width: u32, _height: u32, _tr: &rgui::TextRenderer| {
            let mut layers: Vec<PaintLayerData> = Vec::new();

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

            build_scene_from_paint_data(&layers, frame, Some(_tr))
        },
    );

    println!("=== rgui Slider ===\n");
    println!("窗口: 300x200 | 范围: 0-100 | 默认值: 50");

    app.run()
}
