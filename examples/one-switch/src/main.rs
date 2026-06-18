//! rgui 单 Switch 示例——html! 声明式渲染 + 交互演示。
//!
//! 本示例使用 html! 宏声明 Switch，Arc<AtomicBool> 管理开关状态。

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
            .title("rgui — 单 Switch")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    let switch_bounds = Rect::new(80.0, 80.0, 140.0, 28.0);
    let switch_id = WidgetId::from_u64(1);
    let switch_on = Arc::new(AtomicBool::new(false));

    let on = Arc::clone(&switch_on);
    app.register_interaction(switch_id, switch_bounds, "switch", {
        move |_action| {
            let prev = on.fetch_xor(true, Ordering::Relaxed);
            println!("  Switch 切换: {}", if prev { "开→关" } else { "关→开" });
        }
    });

    let on = Arc::clone(&switch_on);
    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let w = width as f64;
            let h = height as f64;
            let is_on = on.load(Ordering::Relaxed);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <Switch id="1" checked={is_on} />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui 单 Switch 示例 ===\n");
    println!("窗口配置: 300×200 (逻辑像素)");
    println!("点击 Switch 切换开/关状态...");

    app.run()
}
