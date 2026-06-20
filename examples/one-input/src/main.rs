//! one-input — WaInput 组件示例
//!
//! 用 html! 宏声明式展示带 label、placeholder、hint 的 WaInput 组件。
//!
//! WaInput 是 Web Awesome wa-input 的翻译组件，用于单行文本输入。
//! 支持属性: label (标签文本), placeholder (占位符), hint (提示文本),
//!           size (xs/s/m/l/xl), appearance (filled/outlined/filled-outlined),
//!           type (text/password/email/number/search), value (当前值),
//!           disabled (禁用), readonly (只读), required (必填),
//!           pill (药丸形状), with-clear (清除按钮),
//!           password-toggle (密码切换), password-visible (密码可见)。

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
            .title("rgui — WaInput Demo")
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
                        <WaInput label="Username" placeholder="Enter name" size="m" hint="Your full name" />
                    </Column>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaInput Demo ===\n");
    println!("展示: label + placeholder + hint\n");
    app.run()
}
