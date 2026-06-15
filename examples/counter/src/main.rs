//! rgui 计数器——组件 paint() + SceneGraph 桥接演示。
//!
//! 本示例演示完整的渲染管线：
//! 1. 组件状态 → WidgetSpec::paint() → PaintOp
//! 2. PaintOp → SceneGraph（通过 rgui_render::scene_build）
//! 3. SceneGraph → Vello 后端渲染到窗口

use rgui::WidgetSpec;
use rgui::app::{App, AppConfig};
use rgui::{
    Button, ButtonState, Color, Label, LabelState, PaintContext, PaintLayerData, Rect, WidgetId,
    build_scene_from_paint_data,
};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 计数器（组件 paint 集成）")
            .window_size(400.0, 300.0),
    );
    app.register_defaults();

    // 共享状态
    let count = Arc::new(Mutex::new(0_i32));
    let label_state = Arc::new(Mutex::new(LabelState {
        text: "计数: 0".into(),
    }));

    // [+1] 按钮
    let count_clone = Arc::clone(&count);
    let label_clone = Arc::clone(&label_state);
    app.register_interaction(
        WidgetId::from_u64(1),
        Rect::new(50.0, 80.0, 100.0, 40.0),
        "+1",
        move |_| {
            let mut c = count_clone.lock().unwrap();
            *c += 1;
            label_clone.lock().unwrap().text = format!("计数: {}", *c);
            println!("  → {}", *c);
        },
    );

    // [重置] 按钮
    let count_clone2 = Arc::clone(&count);
    let label_clone2 = Arc::clone(&label_state);
    app.register_interaction(
        WidgetId::from_u64(2),
        Rect::new(170.0, 80.0, 100.0, 40.0),
        "reset",
        move |_| {
            *count_clone2.lock().unwrap() = 0;
            label_clone2.lock().unwrap().text = "计数: 0".into();
            println!("  → 重置");
        },
    );

    // 场景构建回调：每帧调用组件 paint() 生成 SceneGraph
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

        // --- 标题 Label ---
        let title_bounds = Rect::new(20.0, 20.0, w - 40.0, 30.0);
        let mut title_ctx = PaintContext::new(title_bounds);
        Label.paint(
            &LabelState {
                text: "rgui 计数器演示".into(),
            },
            title_bounds,
            &mut title_ctx,
        );
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(3),
            0,
            title_bounds,
            title_ctx.into_operations(),
        ));

        // --- 计数 Label ---
        let count_bounds = Rect::new(50.0, 140.0, 300.0, 30.0);
        let mut count_ctx = PaintContext::new(count_bounds);
        let ls = label_state.lock().unwrap();
        Label.paint(&ls, count_bounds, &mut count_ctx);
        drop(ls);
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(4),
            0,
            count_bounds,
            count_ctx.into_operations(),
        ));

        // --- [+1] Button ---
        let btn1_bounds = Rect::new(50.0, 80.0, 100.0, 40.0);
        let mut btn1_ctx = PaintContext::new(btn1_bounds);
        Button.paint(&ButtonState::new("+1"), btn1_bounds, &mut btn1_ctx);
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(1),
            1,
            btn1_bounds,
            btn1_ctx.into_operations(),
        ));

        // --- [重置] Button ---
        let btn2_bounds = Rect::new(170.0, 80.0, 100.0, 40.0);
        let mut btn2_ctx = PaintContext::new(btn2_bounds);
        Button.paint(&ButtonState::new("重置"), btn2_bounds, &mut btn2_ctx);
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(2),
            1,
            btn2_bounds,
            btn2_ctx.into_operations(),
        ));

        build_scene_from_paint_data(&layers, _frame)
    });

    println!("=== rgui 组件 paint() + SceneGraph 桥接演示 ===\n");
    println!("已注册组件: {:?}", app.registry());
    println!("Label 组件名: {}", Label.name());
    println!("Button 组件名: {}", Button.name());
    println!();
    println!("点击窗口按钮： [+1]  [重置]");

    app.run()
}
