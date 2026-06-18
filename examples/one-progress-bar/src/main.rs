//! rgui 单 ProgressBar 示例——最简组件 paint() 演示。
//!
//! 本示例仅绘制一个居中的进度条，用于验证 ProgressBar 组件的
//! 轨道背景 + 填充进度 + 标签文本绘制是否正常工作。

use rgui::app::{App, AppConfig};
use rgui::{
    PaintContext, PaintLayerData, ProgressBar, ProgressBarState, Rect, WidgetId, WidgetSpec,
    build_scene_from_paint_data,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单 ProgressBar")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // 进度条坐标：窗口 300×200 居中
    let pb_bounds = Rect::new(50.0, 85.0, 200.0, 30.0);

    // 视图场景构建回调
    app.set_view_scene_builder(
        move |frame: u64, _width: u32, _height: u32, _tr: &rgui::TextRenderer| {
            let mut layers: Vec<PaintLayerData> = Vec::new();

            // --- 进度条 73% ---
            let mut pb_ctx = PaintContext::new(pb_bounds);
            let mut state = ProgressBarState::new(0.73);
            state.label = "73%".into();
            ProgressBar.paint(&state, pb_bounds, &mut pb_ctx);
            layers.push(PaintLayerData::new(
                WidgetId::from_u64(1),
                0,
                pb_bounds,
                pb_ctx.into_operations(),
            ));

            build_scene_from_paint_data(&layers, frame, None)
        },
    );

    println!("=== rgui 单 ProgressBar 示例 ===\n");
    println!("窗口中显示 73% 进度条");
    println!("ProgressBar 为纯展示组件，无交互");

    app.run()
}
