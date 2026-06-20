//! one-badge — WaBadge 组件示例
//!
//! 用 html! 宏声明式展示不同 variant 和 appearance 的 WaBadge 组件。
//!
//! WaBadge 是 Web Awesome wa-badge 的翻译组件，用于显示状态、计数或标签。
//! 支持属性: variant (brand/neutral/success/warning/danger),
//!           appearance (accent/filled/outlined/filled-outlined),
//!           pill (true/false), label (文本标签)。

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
            .title("rgui — WaBadge Demo")
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
                        <Row gap="8">
                            <WaBadge variant="brand" appearance="accent" pill="true" label="New" />
                            <WaBadge variant="success" appearance="filled" pill="true" label="Done" />
                            <WaBadge variant="warning" appearance="filled" pill="true" label="Warn" />
                        </Row>
                        <Row gap="8">
                            <WaBadge variant="danger" appearance="filled-outlined" pill="true" label="Error" />
                            <WaBadge variant="neutral" appearance="outlined" pill="false" label="Info" />
                            <WaBadge variant="success" appearance="accent" pill="true" label="OK" />
                        </Row>
                    </Column>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaBadge Demo ===\n");
    println!("展示: brand/accent, success/filled, warning/filled,");
    println!("      danger/filled-outlined, neutral/outlined, success/accent\n");
    app.run()
}
