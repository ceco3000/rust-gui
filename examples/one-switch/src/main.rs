//! rgui 单 Switch 示例——开关切换交互演示。
//!
//! 本示例绘制一个带标签的 Switch 开关，点击可切换开/关状态。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rgui::app::{App, AppConfig};
use rgui::{Color, PaintContext, PaintLayerData, Rect, Switch, SwitchState, WidgetId, WidgetSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单 Switch")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    // Switch 坐标：窗口 300×200，居中放置
    let switch_bounds = Rect::new(80.0, 80.0, 140.0, 28.0);
    let switch_id = WidgetId::from_u64(1);

    // Arc<AtomicBool> 在交互闭包和场景构建闭包间共享可变状态
    let switch_on = Arc::new(AtomicBool::new(false));
    let switch_disabled = Arc::new(AtomicBool::new(false));

    let on = Arc::clone(&switch_on);
    let disabled = Arc::clone(&switch_disabled);
    // 交互回调：点击 Switch 区域时切换状态
    app.register_interaction(switch_id, switch_bounds, "switch", {
        move |_action| {
            if !disabled.load(Ordering::Relaxed) {
                let prev = on.fetch_xor(true, Ordering::Relaxed);
                println!("  Switch 切换: {}", if prev { "开→关" } else { "关→开" });
            }
        }
    });

    let on = Arc::clone(&switch_on);
    let disabled = Arc::clone(&switch_disabled);
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

        // --- Switch ---
        let mut switch_ctx = PaintContext::new(switch_bounds);
        let s = SwitchState {
            on: on.load(Ordering::Relaxed),
            disabled: disabled.load(Ordering::Relaxed),
            label: "WiFi".into(),
        };
        Switch.paint(&s, switch_bounds, &mut switch_ctx);
        layers.push(PaintLayerData::new(
            switch_id,
            0,
            switch_bounds,
            switch_ctx.into_operations(),
        ));

        layers
    });

    println!("=== rgui 单 Switch 示例 ===\n");
    println!("窗口配置: 300×200 (逻辑像素)");
    println!("点击 Switch 切换开/关状态...");

    app.run()
}
