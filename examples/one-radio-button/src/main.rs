//! rgui 单 RadioButton 示例——最简组件 paint() 演示。
//!
//! 本示例绘制一个居中的单选按钮（选中状态），用于验证
//! RadioButton 组件的外圈 + 内圆 + 标签文本绘制是否正常。

use rgui::app::{App, AppConfig};
use rgui::{
    PaintContext, PaintLayerData, RadioButton, RadioButtonState, Rect, WidgetId, WidgetSpec,
    build_scene_from_paint_data,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单 RadioButton")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // RadioButton 坐标：窗口 300×200 居中
    let rb_bounds = Rect::new(75.0, 88.0, 150.0, 24.0);

    // RadioButton 交互（点击可切换选中/取消 — 示例中为选中状态）
    app.register_interaction(
        WidgetId::from_u64(1),
        rb_bounds,
        "Option A",
        move |action| {
            println!("  RadioButton 被点击: {action}");
        },
    );

    // 视图场景构建回调
    app.set_view_scene_builder(
        move |frame: u64, _width: u32, _height: u32, _tr: &rgui::TextRenderer| {
            let mut layers: Vec<PaintLayerData> = Vec::new();

            // --- 单选按钮（选中状态） ---
            let mut rb_ctx = PaintContext::new(rb_bounds);
            let mut state = RadioButtonState::new("Option A", "demo");
            state.selected = true;
            RadioButton.paint(&state, rb_bounds, &mut rb_ctx);
            layers.push(PaintLayerData::new(
                WidgetId::from_u64(1),
                0,
                rb_bounds,
                rb_ctx.into_operations(),
            ));

            build_scene_from_paint_data(&layers, frame, None)
        },
    );

    println!("=== rgui 单 RadioButton 示例 ===\n");
    println!("窗口中显示选中的单选按钮 \"Option A\"");
    println!("点击可触发交互");

    app.run()
}
