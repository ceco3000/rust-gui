//! Accordion 组件演示 —— Tier 2 架构（.rgui + .rhai）
//!
//! 展示用声明式格式定义的手风琴容器组件。
//! 交互通过 run_simple_app 内置管线处理。

use rgui::app::{App, AppConfig};
use rgui_core::traits::AppMessage;
use rgui_core::view::WidgetView;
use rgui_render::compute_view_layout;

/// Accordion demo 消息类型
#[derive(Debug, Clone, PartialEq)]
enum AccordionMsg {
    Dummy,
}

impl AppMessage for AccordionMsg {
    fn message_name(&self) -> &'static str {
        "AccordionMsg"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 解析 .rgui + 执行 Tier 2 paint 脚本
    let rgui_path = std::path::PathBuf::from("ui.rgui");
    let mut view: WidgetView<AccordionMsg> =
        rgui_devtools::rgui_parser::parse_rgui_file(&rgui_path)?;
    rgui::paint_factory::execute_tier2_paint_scripts(&mut view);

    // 2. 初始布局
    let _initial_layout = compute_view_layout(
        &mut view,
        rgui_core::geometry::Size::new(500.0, 400.0),
        None,
    );

    // 3. 创建 App — 使用自定义渲染管线
    //    每帧：克隆模板 → 计算布局 → 构建场景
    let config = AppConfig::default()
        .title("Accordion Demo — Tier 2 (.rgui + .rhai)")
        .window_size(500.0, 400.0);

    let mut app = App::new(config);

    let template = view;
    let paint_fn: rgui_render::PaintFn<AccordionMsg> = Box::new(|_view, _bounds| Vec::new());

    app.set_view_scene_builder(move |frame, width, height, tr| {
        let mut v = template.clone();
        let layout = compute_view_layout(
            &mut v,
            rgui_core::geometry::Size::new(f64::from(width), f64::from(height)),
            Some(tr),
        );
        rgui_render::build_scene_from_view(&v, &layout, &paint_fn, frame, Some(tr))
    });

    // 4. 加载 Rhai 事件脚本（占位）
    let rhai_path = std::path::PathBuf::from("handlers.rhai");
    if rhai_path.exists() {
        app.load_rhai_scripts(&[rhai_path.as_path()])?;
    }

    // 5. 启动事件循环
    app.run()
}
