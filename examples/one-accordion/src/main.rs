//! one-accordion — WaAccordion 交互示例
//!
//! WaAccordionItem 内建点击切换展开/折叠行为。
//! 通过 `run_simple_app` 一行初始化，无需手动管理状态或注册交互。

use rgui::AppMessage;
use rgui::app::{run_simple_app, AppConfig};

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum Msg {
    Noop,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== rgui WaAccordion Demo ===\n");
    println!("点击 Section 1 / Section 2 切换展开/折叠\n");

    run_simple_app::<Msg>(AppConfig::new()
        .title("rgui — WaAccordion Demo (click to toggle)")
        .window_size(350.0, 250.0)
        .rgui_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui.rgui"))
        .rhai_paths(vec![
            concat!(env!("CARGO_MANIFEST_DIR"), "/handlers.rhai").into(),
        ]))
}
