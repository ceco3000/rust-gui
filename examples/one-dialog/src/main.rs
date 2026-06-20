//! one-dialog — WaDialog 组件示例
//!
//! 用 html! 宏声明式展示对话框组件。
//!
//! WaDialog 是 Web Awesome wa-dialog 的翻译组件，
//! 支持 label、open、without-header、light-dismiss 属性。

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
            .title("rgui — WaDialog Demo")
            .window_size(400.0, 300.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <WaDialog label="Confirm" open="true" />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaDialog Demo ===\n");
    println!("展示: Confirm 对话框 open=true\n");
    app.run()
}
