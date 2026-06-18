//! rgui 单 RadioButton 示例——html! 声明式渲染演示。
//!
//! 本示例使用 html! 宏声明一个选中的单选按钮。

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
            .title("rgui — 单 RadioButton")
            .window_size(300.0, 200.0),
    );
    app.register_defaults();

    let rb_bounds = Rect::new(75.0, 88.0, 150.0, 24.0);
    app.register_interaction(
        WidgetId::from_u64(1),
        rb_bounds,
        "Option A",
        move |action| {
            println!("  RadioButton 被点击: {action}");
        },
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let w = width as f64;
            let h = height as f64;

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <RadioButton id="1" label="Option A" />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui 单 RadioButton 示例 ===\n");
    println!("窗口中显示选中的单选按钮 \"Option A\"");
    println!("点击可触发交互");

    app.run()
}
