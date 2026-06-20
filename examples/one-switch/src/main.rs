//! one-switch — WaSwitch 组件示例
//!
//! 用 html! 宏声明式展示 on、off、disabled 三种状态的 WaSwitch 组件。
//!
//! WaSwitch 是 Web Awesome wa-switch 的翻译组件，用于显示开关选项。
//! 支持属性: label (标签文本), checked (选中状态),
//!           disabled (禁用状态), size (xs/s/m/l/xl),
//!           required (必填), hint (提示文本)。

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
            .title("rgui — WaSwitch Demo")
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
                        <WaSwitch label="Wi-Fi" checked="true" size="m" />
                        <WaSwitch label="Bluetooth" checked="false" size="m" />
                        <WaSwitch label="Airplane Mode" disabled="true" size="m" />
                    </Column>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaSwitch Demo ===\n");
    println!("展示: on / off / disabled\n");
    app.run()
}
