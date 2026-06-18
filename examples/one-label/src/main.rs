//! rgui 单 Label 示例——html! 声明式渲染演示。
//!
//! 本示例使用 html! 宏声明一个居中文本标签，验证声明式渲染管线。

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
            .title("rgui — 单 Label")
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
                    <Label text="Hello, rgui!" />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui 单 Label 示例 ===\n");
    println!("窗口中显示 \"Hello, rgui!\" 文本标签");
    println!("Label 为纯展示组件，无交互");

    app.run()
}
