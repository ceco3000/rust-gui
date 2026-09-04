//! D10 窗口示例：真实可交互组件——Accordion 折叠/展开（经 facade `rgui::App::run`）。
//!
//! 点击 Accordion 标题（或按 Space）切换展开/收起，展开时显示内容（badge 信息）。
//! 运行：cargo run -p rgui --features window --example window_demo

#![cfg(feature = "window")]

use rgui_platform::event_loop::{ElementState, KeyCode, MouseButton, PhysicalKey, WindowEvent};
use rgui::{Accordion, AccordionMsg, AccordionState, App, AppConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::new()
        .with_title("rgui accordion demo")
        .with_size(480, 320);

    // 事件 → Accordion 消息：左键点击标题区 → Toggle；按 Space → Toggle
    let mapper = |event: &WindowEvent| -> Option<AccordionMsg> {
        match event {
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => Some(AccordionMsg::Toggle),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Space) =>
            {
                Some(AccordionMsg::Toggle)
            }
            _ => None,
        }
    };

    // 初始状态：默认收起；`--expanded` 则初始即展开（qa 截"收起 vs 展开"对比，不依赖辅助功能权限）
    let expanded = std::env::args().any(|a| a == "--expanded");
    let state = AccordionState {
        title: "Settings".to_string(),
        subtitle: "WaBadge: 0 (click to expand details)".to_string(),
        expanded,
    };

    App::run(config, Accordion, state, mapper)?;
    Ok(())
}
