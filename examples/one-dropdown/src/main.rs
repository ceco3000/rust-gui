//! one-dropdown — WaDropdown + WaDropdownItem 组件示例
//!
//! 用 html! 宏声明式展示下拉菜单组件。
//!
//! WaDropdown 是 Web Awesome wa-dropdown 的翻译容器组件，
//! WaDropdownItem 是下拉选项，支持 label、value、disabled 属性。

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
            .title("rgui — WaDropdown Demo")
            .window_size(300.0, 250.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <WaDropdown open="true">
                        <WaDropdownItem label="Option A" value="a" />
                        <WaDropdownItem label="Option B" value="b" />
                        <WaDropdownItem label="Option C" value="c" disabled="true" />
                    </WaDropdown>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaDropdown Demo ===\n");
    println!("展示: 3 个下拉选项，Option C 禁用\n");
    app.run()
}
