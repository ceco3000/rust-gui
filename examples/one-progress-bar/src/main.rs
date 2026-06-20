//! one-progress-bar — WaProgressBar 组件示例
//!
//! 用 html! 宏声明式展示 WaProgressBar 进度条组件。
//!
//! WaProgressBar 是 Web Awesome wa-progress-bar 的翻译组件，用于展示操作进度。
//! 支持属性: value (进度百分比 0-100), indeterminate (不确定状态),
//!           label (辅助标签)。

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
            .title("rgui — WaProgressBar Demo")
            .window_size(300.0, 200.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <Column gap="8">
                        <WaProgressBar value="65" label="Loading" />
                    </Column>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaProgressBar Demo ===\n");
    println!("展示: Loading 进度条 value=65%\n");
    app.run()
}
