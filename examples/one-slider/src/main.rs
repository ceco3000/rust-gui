//! rgui 单 Slider 示例——html! 声明式渲染演示。
//!
//! 本示例使用 html! 宏声明一个范围 0-100 的 Slider，默认值 50。

use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui::{AppMessage, Size, WidgetView, build_scene_from_view, compute_view_layout, html};

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum Msg {
    _Dummy,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — Slider")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let w = width as f64;
            let h = height as f64;

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <Slider value="50.0" min="0.0" max="100.0" />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui Slider ===\n");
    println!("窗口: 300x200 | 范围: 0-100 | 默认值: 50");

    app.run()
}
