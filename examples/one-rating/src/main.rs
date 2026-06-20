//! one-rating — WaRating 组件示例
//!
//! 用 html! 宏声明式展示 WaRating 评分组件。
//!
//! WaRating 是 Web Awesome wa-rating 的翻译组件，用星星符号展示评分。
//! 支持属性: value (当前值), max (最大值), precision (精度),
//!           label (无障碍标签), size (尺寸), disabled (禁用),
//!           readonly (只读), required (必填), name (表单字段名)。

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
            .title("rgui — WaRating Demo")
            .window_size(300.0, 200.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <WaRating value="4" max="5" label="Rating" />
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaRating Demo ===\n");
    println!("展示: Rating 评分 value=4 max=5\n");
    app.run()
}
