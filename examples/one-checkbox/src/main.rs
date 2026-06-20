//! one-checkbox — WaCheckbox 组件示例
//!
//! 用 html! 宏声明式展示 checked、unchecked、disabled、indeterminate
//! 四种状态的 WaCheckbox 组件。
//!
//! WaCheckbox 是 Web Awesome wa-checkbox 的翻译组件，用于显示复选框选项。
//! 支持属性: label (标签文本), checked (选中状态),
//!           disabled (禁用状态), indeterminate (半选状态),
//!           size (xs/s/m/l/xl), required (必填),
//!           hint (提示文本)。

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
            .title("rgui — WaCheckbox Demo")
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
                        <WaCheckbox label="Accept terms" checked="true" size="m" />
                        <WaCheckbox label="Opt in to newsletter" checked="false" size="m" />
                        <WaCheckbox label="Disabled option" disabled="true" size="m" />
                        <WaCheckbox label="Select all" indeterminate="true" size="m" />
                    </Column>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaCheckbox Demo ===\n");
    println!("展示: checked / unchecked / disabled / indeterminate\n");
    app.run()
}
