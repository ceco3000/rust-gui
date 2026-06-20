//! one-accordion — WaAccordion + WaAccordionItem 组件示例
//!
//! 用 html! 宏声明式展示手风琴/折叠面板组件。
//!
//! WaAccordion 是 Web Awesome wa-accordion 的翻译容器组件，
//! WaAccordionItem 是可展开/折叠的面板项，支持 label、expanded 属性。

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
            .title("rgui — WaAccordion Demo")
            .window_size(350.0, 250.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <WaAccordion>
                        <WaAccordionItem label="Section 1" expanded="true" />
                        <WaAccordionItem label="Section 2" />
                    </WaAccordion>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaAccordion Demo ===\n");
    println!("展示: Section 1 默认展开, Section 2 折叠\n");
    app.run()
}
