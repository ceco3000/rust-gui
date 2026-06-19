//! one-rgui-button — .rgui + .rhai 声明式按钮示例
//!
//! 演示完整的 `.rgui` 声明式 UI + `.rhai` 事件处理工作流：
//! 1. `ui.rgui` — XML 风格 UI 声明（Button + onclick）
//! 2. `handlers.rhai` — Rhai 脚本事件处理器
//! 3. `main.rs` — 解析、布局、交互注册、渲染、运行

use rgui::AppMessage;
use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui_core::geometry::{Rect, Size};
use rgui_devtools::rgui_parser::parse_rgui_file;
use rgui_layout::LayoutEngine;
use rgui_render::{build_scene_from_view, compute_view_layout};
use std::path::Path;

// ============================================================================
// 消息类型（.rgui 解析器需要的泛型参数）
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum Msg {
    Noop,
}

// ============================================================================
// 辅助：遍历 WidgetView 树，为带 onclick 的 widget 注册交互
// ============================================================================

fn register_click_interactions<M: AppMessage>(
    view: &rgui_core::view::WidgetView<M>,
    engine: &LayoutEngine,
    app: &mut App,
) {
    if let Some(widget_id) = view.id {
        // 检查是否有 onclick 属性
        if let Some(action) = view.props.get("onclick") {
            if let rgui_core::view::PropValue::Str(action_str) = action {
                // 获取该 widget 的布局矩形
                if let Some(layout) = engine.get_layout(widget_id) {
                    let rect = Rect::new(
                        layout.result.position.x,
                        layout.result.position.y,
                        layout.result.size.width,
                        layout.result.size.height,
                    );
                    let action_owned = action_str.to_string();
                    app.register_interaction(widget_id, rect, &action_owned, move |_| {
                        // Rhai 路由在 handle_click 中处理，此回调为空
                    });
                }
            }
        }
    }

    // 递归子节点
    for child in &view.children {
        register_click_interactions(child, engine, app);
    }
}

// ============================================================================
// 主函数
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 解析 .rgui → WidgetView 树
    let rgui_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui.rgui"));
    let mut view: rgui_core::view::WidgetView<Msg> =
        parse_rgui_file(rgui_path).map_err(|e| format!(".rgui 解析失败: {e}"))?;

    // 2. 计算布局 → 分配 WidgetIds + 布局结果
    let layout = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

    // 3. 创建 App 并注册交互区域（onclick → interaction）
    let config = AppConfig::new()
        .title("rgui — .rgui + .rhai Button")
        .window_size(400.0, 300.0);

    let mut app = App::new(config);
    register_click_interactions(&view, &layout, &mut app);

    // 4. 加载 .rhai 脚本（启动热重载 + 注册 Rhai 函数）
    let rhai_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/handlers.rhai"));
    app.load_rhai_scripts(&[rhai_path])
        .map_err(|e| format!(".rhai 加载失败: {e}"))?;

    // 5. 设置声明式渲染回调（每帧从 WidgetView 构建 SceneGraph）
    let current_view = view;
    let paint_fn = default_paint_fn::<Msg>();
    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            let mut v = current_view.clone();
            let l = compute_view_layout(
                &mut v,
                Size::new(f64::from(width), f64::from(height)),
                Some(tr),
            );
            build_scene_from_view(&v, &l, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui .rgui + .rhai 按钮示例 ===\n");
    println!("UI: ui.rgui  →  <Button label=\"Click Me\" onclick=\"handle_click\"/>");
    println!("脚本: handlers.rhai  →  fn handle_click() {{ print(\"...\"); }}");
    println!();
    println!("点击按钮 → Rhai 脚本执行 → 终端输出：");
    println!("  🎯 rgui button clicked via Rhai!\n");

    app.run()
}
