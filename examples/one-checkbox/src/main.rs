//! rgui 单 CheckBox 示例——复选框交互演示。
//!
//! 本示例绘制一个带标签的 CheckBox，点击可切换选中/未选中状态。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rgui::app::{App, AppConfig};
use rgui::{
    CheckBox, CheckBoxState, PaintContext, PaintLayerData, Rect, WidgetId, WidgetSpec,
    build_scene_from_paint_data,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单 CheckBox")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // CheckBox 坐标：窗口 300×200，居中放置
    let cb_bounds = Rect::new(60.0, 85.0, 180.0, 28.0);
    let cb_id = WidgetId::from_u64(1);

    // Arc<AtomicBool> 在交互闭包和场景构建闭包间共享
    let checked = Arc::new(AtomicBool::new(false));
    let disabled = Arc::new(AtomicBool::new(false));

    let c = Arc::clone(&checked);
    let d = Arc::clone(&disabled);
    // 交互回调：点击 CheckBox 区域时切换选中状态
    app.register_interaction(cb_id, cb_bounds, "checkbox", {
        move |_action| {
            if !d.load(Ordering::Relaxed) {
                let prev = c.fetch_xor(true, Ordering::Relaxed);
                println!(
                    "  CheckBox toggle: {}",
                    if prev {
                        "checked -> unchecked"
                    } else {
                        "unchecked -> checked"
                    }
                );
            }
        }
    });

    let c = Arc::clone(&checked);
    let d = Arc::clone(&disabled);
    // 视图场景构建回调
    app.set_view_scene_builder(move |frame: u64, _width: u32, _height: u32| {
        let mut layers: Vec<PaintLayerData> = Vec::new();

        // --- CheckBox ---
        let mut cb_ctx = PaintContext::new(cb_bounds);
        let s = CheckBoxState {
            checked: c.load(Ordering::Relaxed),
            disabled: d.load(Ordering::Relaxed),
            label: "I agree to the terms".into(),
        };
        CheckBox.paint(&s, cb_bounds, &mut cb_ctx);
        layers.push(PaintLayerData::new(
            cb_id,
            0,
            cb_bounds,
            cb_ctx.into_operations(),
        ));

        build_scene_from_paint_data(&layers, frame, None)
    });

    println!("=== rgui 单 CheckBox 示例 ===\n");
    println!("窗口配置: 300x200 (逻辑像素)");
    println!("点击 CheckBox 切换选中/未选中状态...");

    app.run()
}
