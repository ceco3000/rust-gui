//! rgui 单 TextField 示例——文本输入框演示。
//!
//! 本示例绘制一个带占位符的 TextField，点击可聚焦。

use rgui::app::{App, AppConfig};
use rgui::{
    PaintContext, PaintLayerData, Rect, TextField, TextFieldState, WidgetId, WidgetSpec,
    build_scene_from_paint_data,
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

    // 视图场景构建回调
    app.set_view_scene_builder(
        move |frame: u64, _width: u32, _height: u32, _tr: &rgui::TextRenderer| {
            let mut layers: Vec<PaintLayerData> = Vec::new();

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

            build_scene_from_paint_data(&layers, frame, None)
        },
    );

    println!("=== rgui TextField ===\n");
    println!("窗口: 300x200 | 占位符: 'Enter text...'");

    app.run()
}
