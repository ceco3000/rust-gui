//! rgui 单按钮示例——最简组件 paint() 演示。
//!
//! 本示例仅绘制一个居中的 "OK" 按钮，用于验证 Button 组件的
//! 背景 + 文本绘制是否正常工作。

use rgui::app::{App, AppConfig};
use rgui::{Button, ButtonState, Color, PaintContext, PaintLayerData, Rect, WidgetId, WidgetSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单按钮（OK）")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // 按钮坐标：窗口 300×200 居中 (100×40 按钮)
    let btn_bounds = Rect::new(100.0, 80.0, 100.0, 40.0);

    // 按钮交互
    app.register_interaction(WidgetId::from_u64(1), btn_bounds, "OK", move |action| {
        println!("  按钮被点击: {action}");
    });

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

        // --- OK 按钮 ---
        let mut btn_ctx = PaintContext::new(btn_bounds);
        Button.paint(&ButtonState::new("OK"), btn_bounds, &mut btn_ctx);
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            btn_bounds,
            btn_ctx.into_operations(),
        ));

        layers
    });

    println!("=== rgui 单按钮示例 ===\n");
    println!("点击窗口中的 [OK] 按钮...");

    app.run()
}
