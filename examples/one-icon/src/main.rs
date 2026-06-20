//! one-icon — WaIcon 组件示例
//!
//! 用 html! 宏声明式展示不同尺寸和名称的 WaIcon 组件。
//!
//! WaIcon 是 Web Awesome wa-icon 的翻译组件，使用 Unicode 字符渲染矢量图标。
//! 支持属性: name (图标名), size (s/m/l), label (无障碍标签)。

use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui::{build_scene_from_view, compute_view_layout, html, AppMessage, Size, WidgetView};

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum Msg {
    _Dummy,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — WaIcon Demo")
            .window_size(300.0, 200.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <Row gap="12">
                        <WaIcon name="check" size="s" />
                        <WaIcon name="check" size="m" />
                        <WaIcon name="check" size="l" />
                        <WaIcon name="star" size="m" />
                        <WaIcon name="xmark" size="m" />
                    </Row>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaIcon Demo ===\n");
    println!("展示: check (s/m/l), star, xmark\n");
    app.run()
}
