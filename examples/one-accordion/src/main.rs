//! one-accordion — WaAccordion 深度演示
//!
//! 展示三种不同 appearance + mode + icon-placement 变体。
//! 通过 `run_simple_app` 一行初始化，组件自己管理交互状态。

use rgui::AppMessage;
use rgui::app::{AppConfig, run_simple_app};

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum Msg {
    Noop,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== rgui WaAccordion Demo ===\n");
    println!("3 个 Accordion 展示：");
    println!("  1. outlined / single / icon-end (3 sections)");
    println!("  2. filled / multiple / icon-end (3 sections, 1 disabled)");
    println!("  3. plain / single-collapsible / icon-start (2 sections)");
    println!("\n点击标题栏切换展开/折叠。关闭窗口退出。\n");

    run_simple_app::<Msg>(
        AppConfig::new()
            .title("rgui — WaAccordion Demo")
            .window_size(480.0, 820.0)
            .rgui_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui.rgui"))
            .rhai_paths(vec![
                concat!(env!("CARGO_MANIFEST_DIR"), "/handlers.rhai").into(),
            ]),
    )
}
