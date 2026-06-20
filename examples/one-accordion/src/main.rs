//! one-accordion — WaAccordion 交互示例 (.rgui + 状态)
//!
//! UI 由 ui.rgui 声明，点击事件通过 Arc<AtomicBool> 切换展开/折叠状态，
//! 每帧根据状态动态注入 expanded prop。

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rgui::AppMessage;
use rgui::app::{App, AppConfig};
use rgui::paint_factory::default_paint_fn;
use rgui_core::geometry::{Rect, Size};
use rgui_core::view::PropValue;
use rgui_devtools::rgui_parser::parse_rgui_file;
use rgui_layout::LayoutEngine;
use rgui_render::{build_scene_from_view, compute_view_layout};

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum Msg {
    Noop,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rgui_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui.rgui"));
    let mut view: rgui_core::view::WidgetView<Msg> = parse_rgui_file(rgui_path)?;

    // 两个 section 的展开状态
    let s1_expanded = Arc::new(AtomicBool::new(true));
    let s2_expanded = Arc::new(AtomicBool::new(false));

    // 初始布局（用于获取 WidgetId + Rect，注册交互）
    let initial_layout = compute_view_layout(&mut view, Size::new(350.0, 250.0), None);

    // 创建 App
    let config = AppConfig::new()
        .title("rgui — WaAccordion Demo (click to toggle)")
        .window_size(350.0, 250.0);
    let mut app = App::new(config);

    // 为 WaAccordionItem 注册点击交互 —— 在回调中 toggle AtomicBool
    register_accordion_clicks(&view, &initial_layout, &mut app, &s1_expanded, &s2_expanded);

    // 加载 Rhai（空脚本，占位）
    let rhai_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/handlers.rhai"));
    app.load_rhai_scripts(&[rhai_path])?;

    // 渲染回调：每帧根据 AtomicBool 状态动态注入 expanded prop
    let paint_fn = default_paint_fn::<Msg>();
    let template = view; // 静态模板
    let s1 = Arc::clone(&s1_expanded);
    let s2 = Arc::clone(&s2_expanded);

    app.set_view_scene_builder(
        move |frame: u64, width: u32, height: u32, tr: &rgui::TextRenderer| {
            // 从模板克隆，注入动态 expanded prop
            let mut v = template.clone();
            inject_expanded_state(
                &mut v,
                s1.load(Ordering::Relaxed),
                s2.load(Ordering::Relaxed),
            );

            let l = compute_view_layout(
                &mut v,
                Size::new(f64::from(width), f64::from(height)),
                Some(tr),
            );
            build_scene_from_view(&v, &l, &paint_fn, frame, Some(tr))
        },
    );

    println!("=== rgui WaAccordion Demo ===\n");
    println!("点击 Section 1 / Section 2 切换展开/折叠\n");
    app.run()
}

/// 遍历 WidgetView 树，找到 WaAccordionItem 并注入当前 expanded 状态。
fn inject_expanded_state<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    e1: bool,
    e2: bool,
) {
    if view.widget_type == "WaAccordionItem" {
        let label = match view.props.get("label") {
            Some(PropValue::Str(s)) => s.as_ref(),
            _ => "",
        };
        let expanded = if label == "Section 1" { e1 } else { e2 };
        view.props
            .insert("expanded", PropValue::Str(expanded.to_string().into()));
    }
    for child in &mut view.children {
        inject_expanded_state(child, e1, e2);
    }
}

/// 遍历 WidgetView 树，为 WaAccordionItem 注册点击交互。
fn register_accordion_clicks<M: AppMessage>(
    view: &rgui_core::view::WidgetView<M>,
    engine: &LayoutEngine,
    app: &mut App,
    s1: &Arc<AtomicBool>,
    s2: &Arc<AtomicBool>,
) {
    if view.widget_type == "WaAccordionItem" {
        if let Some(widget_id) = view.id {
            if let Some(l) = engine.get_layout(widget_id) {
                let rect = Rect::new(
                    l.result.position.x,
                    l.result.position.y,
                    l.result.size.width,
                    l.result.size.height,
                );
                let label: String = match view.props.get("label") {
                    Some(PropValue::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                let toggle = if label == "Section 1" {
                    Arc::clone(s1)
                } else {
                    Arc::clone(s2)
                };
                let label_clone = label.clone();
                app.register_interaction(widget_id, rect, "toggle", move |_| {
                    toggle.fetch_xor(true, Ordering::Relaxed);
                    println!("🔄 {} toggled", label_clone);
                });
            }
        }
    }
    for child in &view.children {
        register_accordion_clicks(child, engine, app, s1, s2);
    }
}
