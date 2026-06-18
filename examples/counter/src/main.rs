//! rgui 计数器——使用 `html!` 声明式语法 + `build_scene_from_view` 渲染。
//!
//! 本示例演示 `html!` 宏构建 WidgetView 树，
//! 并通过 `build_scene_from_view` 直接渲染到 SceneGraph。

use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui::{
    AppMessage, Color, PaintContext, PaintLayerData, Rect, WidgetId, WidgetView,
    build_scene_from_paint_data, build_scene_from_view, html,
};
use std::sync::{Arc, Mutex};

// ============================================================================
// 消息类型
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum CounterMsg {
    Increment,
    Reset,
}

// ============================================================================
// 主函数
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 计数器（html! 声明式渲染）")
            .window_size(400.0, 300.0),
    );
    app.register_defaults();

    let count = Arc::new(Mutex::new(0_i32));

    // [+1] 按钮交互
    let c1 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(1),
        Rect::new(50.0, 80.0, 100.0, 40.0),
        "+1",
        move |_| {
            *c1.lock().unwrap() += 1;
        },
    );

    // [重置] 按钮交互
    let c2 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(2),
        Rect::new(170.0, 80.0, 100.0, 40.0),
        "reset",
        move |_| {
            *c2.lock().unwrap() = 0;
        },
    );

    // 视图场景构建回调——使用 html! 声明式 UI + build_scene_from_view 渲染
    let count_for_view = Arc::clone(&count);
    let paint_fn = default_paint_fn();
    app.set_view_scene_builder(move |frame: u64, width: u32, height: u32| {
        let w = width as f64;
        let h = height as f64;
        let cnt = *count_for_view.lock().unwrap();

        // html! 声明式 UI 定义
        let view: WidgetView<CounterMsg> = html! {
            <Column>
                <Label text="rgui 计数器演示" />
                <Row gap="8.0">
                    <Button id="1" label="+1" on:click={CounterMsg::Increment} />
                    <Button id="2" label="重置" on:click={CounterMsg::Reset} />
                </Row>
                <Label text={format!("计数: {}", cnt)} />
            </Column>
        };

        // 背景层（使用 PaintLayerData 手动构建）
        let mut bg_ctx = PaintContext::new(Rect::new(0.0, 0.0, w, h));
        bg_ctx.fill_rect(
            Rect::new(0.0, 0.0, w, h),
            Color::new(14.0 / 255.0, 18.0 / 255.0, 28.0 / 255.0, 1.0),
            0.0,
        );
        let bg_layer = PaintLayerData::new(
            WidgetId::from_u64(0),
            -1,
            Rect::new(0.0, 0.0, w, h),
            bg_ctx.into_operations(),
        );

        let mut bg_scene = build_scene_from_paint_data(&[bg_layer], frame, None);

        // 从 WidgetView 树构建 SceneGraph
        let view_scene =
            build_scene_from_view(&view, Rect::new(0.0, 0.0, w, h), &paint_fn, frame, None);

        // 合并：背景层在前，视图层在后
        bg_scene.layers.extend(view_scene.layers);
        bg_scene
    });

    println!("=== rgui 计数器（html! 声明式渲染）===\n");
    println!("UI 由 html! 宏声明，通过 build_scene_from_view 直接渲染。");
    println!();
    println!("点击窗口按钮： [+1]  [重置]");

    app.run()
}
