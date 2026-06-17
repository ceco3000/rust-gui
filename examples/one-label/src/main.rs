//! rgui 单 Label 示例——最简组件 paint() 演示。
//!
//! 本示例仅绘制一个居中的文本标签，用于验证 Label 组件的
//! 文本绘制是否正常工作。

use rgui::app::{App, AppConfig};
use rgui::{Color, Label, LabelState, PaintContext, PaintLayerData, Rect, WidgetId, WidgetSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单 Label")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // Label 坐标：窗口 300×200 居中
    let label_bounds = Rect::new(60.0, 85.0, 180.0, 30.0);

    // 场景构建回调
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

        // --- 文本标签 ---
        let mut label_ctx = PaintContext::new(label_bounds);
        let state = LabelState {
            text: "Hello, rgui!".into(),
        };
        Label.paint(&state, label_bounds, &mut label_ctx);
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            label_bounds,
            label_ctx.into_operations(),
        ));

        layers
    });

    println!("=== rgui 单 Label 示例 ===\n");
    println!("窗口中显示 \"Hello, rgui!\" 文本标签");
    println!("Label 为纯展示组件，无交互");

    app.run()
}
