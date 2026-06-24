//! Accordion 组件演示 —— Tier 2 架构（.rgui + .rhai）
//!
//! 翻译自 WA accordion + accordion-item web components。
//! 交互（expand/collapse toggle + mode 协调）由框架自动处理，
//! 用户只需声明式 UI 和一行 `run_simple_app` 启动器。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rgui::run_simple_app::<rgui_core::message::NoopMsg>(
        rgui::AppConfig::default()
            .title("Accordion Demo — Tier 2 (.rgui + .rhai)")
            .window_size(500.0, 400.0)
            .rgui_path("ui.rgui"),
    )
}
