//! rgui 单 CheckBox 示例——html! 声明式渲染 + 交互演示。
//!
//! 本示例使用 html! 宏声明 CheckBox，Arc<AtomicBool> 管理选中状态。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui::{
    AppMessage, Rect, Size, WidgetId, WidgetView, build_scene_from_view, compute_view_layout, html,
};

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum Msg {
    _Dummy,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 单 CheckBox")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    let checked = Arc::new(AtomicBool::new(false));
    let cb_bounds = Rect::new(60.0, 85.0, 180.0, 28.0);
    let cb_id = WidgetId::from_u64(1);

    let c = Arc::clone(&checked);
    app.register_interaction(cb_id, cb_bounds, "checkbox", {
        move |_action| {
            let prev = c.fetch_xor(true, Ordering::Relaxed);
            println!(
                "  CheckBox toggle: {}",
                if prev {
                    "checked → unchecked"
                } else {
                    "unchecked → checked"
                }
            );
        }
    });

    let c = Arc::clone(&checked);
    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let w = width as f64;
            let h = height as f64;
            let is_checked = c.load(Ordering::Relaxed);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <CheckBox id="1" label="I agree to the terms" checked={is_checked} />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui 单 CheckBox 示例 ===\n");
    println!("窗口配置: 300x200 (逻辑像素)");
    println!("点击 CheckBox 切换选中/未选中状态...");

    app.run()
}
