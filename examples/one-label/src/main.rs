//! rgui 单 Label 示例——最简组件 paint() 演示。
//!
//! 本示例仅绘制一个居中的文本标签，用于验证 Label 组件的
//! 文本绘制是否正常工作。

use rgui::app::{App, AppConfig};
use rgui::{
    Label, LabelState, PaintContext, PaintLayerData, Rect, WidgetId, WidgetSpec,
    build_scene_from_paint_data,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单 Label")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // Label 坐标：窗口 300×200 居中
    let label_bounds = Rect::new(60.0, 85.0, 180.0, 30.0);

    // 视图场景构建回调
    app.set_view_scene_builder(
        move |frame: u64, _width: u32, _height: u32, _tr: &rgui::TextRenderer| {
            let mut layers: Vec<PaintLayerData> = Vec::new();

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

            build_scene_from_paint_data(&layers, frame, Some(_tr))
        },
    );

    println!("=== rgui 单 Label 示例 ===\n");
    println!("窗口中显示 \"Hello, rgui!\" 文本标签");
    println!("Label 为纯展示组件，无交互");

    app.run()
}
