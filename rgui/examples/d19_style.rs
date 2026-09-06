//! D19 示例：样式驱动——自定义样式表覆盖组件配色（Accordion 背景红 Vs 默认蓝）。
//!
//! 运行：cargo run -p rgui --features window --example d19_style

#![cfg(feature = "window")]

use rgui::style::{default_style, StyleProperties, StyleSheet};
use rgui::view::Color;
use rgui::{App, AppConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 自定义样式表（D23）：主题色改用系统语义色 systemBlue/systemTeal + 深前景（保证对比度 ≥4.5:1，#101010/#007AFF≈4.66:1）
    let custom: &'static StyleSheet = Box::leak(Box::new(
        StyleSheet::new()
            .rule(
                "accordion",
                StyleProperties::new()
                    .background(Color::rgb(0, 122, 255)) // systemBlue
                    .foreground(Color::rgb(16, 16, 16)), // 深前景（浅底配深字）
            )
            .rule("accordion", StyleProperties::new().border_pad(6.0))
            .rule(
                "wa_badge",
                StyleProperties::new()
                    .background(Color::rgb(90, 200, 250)) // systemTeal
                    .foreground(Color::rgb(16, 16, 16)),
            ),
    ));

    // 对比：默认主题（蓝色）作为基准截图（第二条运行）。
    let _default = default_style();

    let config = AppConfig::new()
        .with_title("rgui d19 style (styled)")
        .with_size(520, 220)
        .with_stylesheet(custom);

    let mapper = move |event: &rgui_platform::event_loop::WindowEvent| -> Option<rgui::components::AccordionMsg> {
        match event {
            rgui_platform::event_loop::WindowEvent::MouseInput {
                state: rgui_platform::event_loop::ElementState::Pressed,
                ..
            } => Some(rgui::components::AccordionMsg::Toggle),
            _ => None,
        }
    };

    App::run(
        config,
        rgui::components::Accordion,
        rgui::components::AccordionState::default(),
        mapper,
    )?;
    Ok(())
}
