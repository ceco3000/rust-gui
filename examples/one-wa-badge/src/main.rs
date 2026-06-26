//! WaBadge 组件演示 —— Tier 1 WidgetSpec 架构
//!
//! 翻译自 WA badge web component。
//! 组件为 NoOp 消息，无交互。用户只需声明式 UI 和一行 `run_simple_app` 启动器。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rgui::init_logging();

    let rgui_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui.rgui");

    rgui::run_simple_app::<rgui_core::message::NoopMsg>(
        rgui::AppConfig::default()
            .title("WaBadge Demo — Tier 1 (WidgetSpec)")
            .window_size(400.0, 300.0)
            .rgui_path(&rgui_path),
    )
}
