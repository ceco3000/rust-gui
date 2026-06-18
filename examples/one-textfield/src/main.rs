//! rgui 单 TextField 示例——文本输入框演示。
//!
//! 本示例绘制一个带占位符的 TextField，点击可聚焦。

use rgui::app::{App, AppConfig};
use rgui::{
    Color, PaintContext, PaintLayerData, Rect, TextField, TextFieldState, WidgetId, WidgetSpec,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — TextField")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // TextField 坐标：窗口 300x200，居中
    let tf_bounds = Rect::new(50.0, 85.0, 200.0, 32.0);
    let tf_id = WidgetId::from_u64(1);

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

        // --- TextField ---
        let mut tf_ctx = PaintContext::new(tf_bounds);
        // 显示占位符文本（无实际输入内容时）
        let s = TextFieldState {
            placeholder: "Enter text...".into(),
            ..Default::default()
        };
        TextField.paint(&s, tf_bounds, &mut tf_ctx);
        layers.push(PaintLayerData::new(
            tf_id,
            0,
            tf_bounds,
            tf_ctx.into_operations(),
        ));

        layers
    });

    println!("=== rgui TextField ===\n");
    println!("窗口: 300x200 | 占位符: 'Enter text...'");

    app.run()
}
