//! one-slider — WaSlider 组件示例
//!
//! 用 html! 宏声明式展示 WaSlider 滑块组件。
//!
//! WaSlider 是 Web Awesome wa-slider 的翻译组件，用于选择数值范围。
//! 支持属性: label (标签文本), value (当前值), min (最小值),
//!           max (最大值), step (步长), size (尺寸),
//!           orientation (方向), disabled (禁用), hint (提示文本)。

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
            .title("rgui — WaSlider Demo")
            .window_size(350.0, 200.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <Column gap="8">
                        <WaSlider label="Volume" value="75" min="0" max="100" />
                    </Column>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaSlider Demo ===\n");
    println!("展示: Volume 滑块 value=75 min=0 max=100\n");
    app.run()
}
