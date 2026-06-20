//! one-tree — WaTree + WaTreeItem 组件示例
//!
//! 用 html! 宏声明式展示树形控件组件。
//!
//! WaTree 是 Web Awesome wa-tree 的翻译容器组件，
//! WaTreeItem 是树节点，支持 label、expanded、is-leaf、depth 属性。

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
            .title("rgui — WaTree Demo")
            .window_size(350.0, 300.0),
    );

    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, _tr: &rgui::TextRenderer| {
            let w = f64::from(width);
            let h = f64::from(height);

            let mut view: WidgetView<Msg> = html! {
                <Center>
                    <WaTree>
                        <WaTreeItem label="Documents" expanded="true" is_leaf="false" depth="0">
                            <WaTreeItem label="Work" is_leaf="true" depth="1" />
                            <WaTreeItem label="Personal" is_leaf="true" depth="1" />
                        </WaTreeItem>
                    </WaTree>
                </Center>
            };

            let layout = compute_view_layout(&mut view, Size::new(w, h), Some(_tr));
            build_scene_from_view(&view, &layout, &paint_fn, frame, Some(_tr))
        },
    );

    println!("=== rgui WaTree Demo ===\n");
    println!("展示: Documents 展开显示 Work 和 Personal 子节点\n");
    app.run()
}
