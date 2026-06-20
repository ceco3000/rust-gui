//! one-breadcrumb — WaBreadcrumb + WaBreadcrumbItem 组件示例
//!
//! 用 html! 宏声明式展示面包屑导航组件。
//!
//! WaBreadcrumb 是 Web Awesome wa-breadcrumb 的翻译容器组件，
//! WaBreadcrumbItem 是单个面包屑项，支持 label、href、separator 属性。

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
            .title("rgui — WaBreadcrumb Demo")
            .window_size(350.0, 200.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <WaBreadcrumb label="Breadcrumb navigation">
                        <WaBreadcrumbItem label="Home" href="#home" separator="true" />
                        <WaBreadcrumbItem label="Products" href="#products" separator="true" />
                        <WaBreadcrumbItem label="Details" separator="false" />
                    </WaBreadcrumb>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaBreadcrumb Demo ===\n");
    println!("展示: Home > Products > Details (当前页无分隔符)\n");
    app.run()
}
