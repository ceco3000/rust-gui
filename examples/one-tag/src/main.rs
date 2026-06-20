//! one-tag — WaTag 组件示例
//!
//! 用 html! 宏声明式展示不同 variant 的 WaTag 标签组件。
//!
//! WaTag 是 Web Awesome wa-tag 的翻译组件，用于显示状态、分类或可选标记。
//! 支持属性: variant (brand/neutral/success/warning/danger),
//!           appearance (accent/filled/outlined/filled-outlined),
//!           size (尺寸), pill (全圆角), with_remove (可移除),
//!           label (文本标签)。

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
            .title("rgui — WaTag Demo")
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
                            <WaTag variant="brand" appearance="accent" label="Brand" size="m" />
                            <WaTag variant="neutral" appearance="filled" label="Neutral" size="m" />
                            <WaTag variant="success" appearance="filled-outlined" label="Success" size="m" />
                        </Row>
                        <Row gap="8">
                            <WaTag variant="warning" appearance="filled" label="Warning" size="m" />
                            <WaTag variant="danger" appearance="filled-outlined" label="Danger" size="m" />
                            <WaTag variant="success" appearance="accent" label="Completed" size="m" />
                        </Row>
                    </Column>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaTag Demo ===\n");
    println!("展示: brand, neutral, success, warning, danger, success/accent\n");
    app.run()
}
