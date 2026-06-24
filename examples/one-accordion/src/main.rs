//! Accordion 组件演示 —— Tier 2 架构（.rgui + .rhai）
//!
//! 展示用声明式格式定义的手风琴容器组件。
//! AC07: 点击标题栏 → expanded 状态翻转 → paint 脚本重新执行。
//!
//! 交互通过 `register_interaction` 注册点击回调。

use rgui::app::{App, AppConfig};
use rgui_core::geometry::Rect;
use rgui_core::traits::AppMessage;
use rgui_core::view::{PropValue, WidgetView};
use rgui_render::compute_view_layout;
use std::sync::{Arc, Mutex};

/// Accordion demo 消息类型
#[derive(Debug, Clone, PartialEq)]
enum AccordionMsg {
    /// Toggle 指定 index 的 AccordionItem expanded 状态
    Toggle(usize),
}

impl AppMessage for AccordionMsg {
    fn message_name(&self) -> &'static str {
        match self {
            AccordionMsg::Toggle(_) => "AccordionMsg::Toggle",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 解析 .rgui
    let rgui_path = std::path::PathBuf::from("ui.rgui");
    let mut view: WidgetView<AccordionMsg> =
        rgui_devtools::rgui_parser::parse_rgui_file(&rgui_path)?;

    // 2. 初始布局（Tier 2 脚本需要 bounds，AC02）
    let initial_layout = compute_view_layout(
        &mut view,
        rgui_core::geometry::Size::new(500.0, 400.0),
        None,
    );

    // 3. 执行 Tier 2 Rhai paint 脚本
    rgui::paint_factory::execute_tier2_paint_scripts(&mut view, &initial_layout);

    // 4. 收集 AccordionItem 的 WidgetId（用于交互注册）
    let item_ids: Vec<_> = collect_accordion_item_ids(&view);

    // 5. 创建 App
    let config = AppConfig::default()
        .title("Accordion Demo — Tier 2 (.rgui + .rhai)")
        .window_size(500.0, 400.0);

    let mut app = App::new(config);

    // 5a. 注册 AccordionItem 点击交互
    let expanded_states: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![
        true,  // Item 0 (Getting Started) 初始展开
        false, // Item 1 (Installation) 初始折叠
        false, // Item 2 (API Reference) 初始折叠（disabled）
    ]));

    for (i, &item_id) in item_ids.iter().enumerate() {
        let states = Arc::clone(&expanded_states);
        // 从布局引擎获取 item 的 bounds
        let item_rect = item_bounds(&initial_layout, item_id);

        app.register_interaction(item_id, item_rect, "toggle", move |_action| {
            if let Ok(mut expanded) = states.lock() {
                if i < expanded.len() {
                    expanded[i] = !expanded[i];
                }
            }
        });
    }

    // 6. 设置视图场景构建器（每帧回调）
    let template = view;
    let paint_fn: rgui_render::PaintFn<AccordionMsg> = Box::new(|_view, _bounds| Vec::new());
    let states = Arc::clone(&expanded_states);

    app.set_view_scene_builder(move |frame, width, height, tr| {
        let mut v = template.clone();
        let layout = compute_view_layout(
            &mut v,
            rgui_core::geometry::Size::new(f64::from(width), f64::from(height)),
            Some(tr),
        );

        // AC07: 注入 expanded props（从 shared state 读取最新值）
        if let Ok(expanded) = states.lock() {
            inject_expanded_props(&mut v, &expanded);
        }

        // AC07: 重新执行 Tier 2 paint 脚本
        rgui::paint_factory::execute_tier2_paint_scripts(&mut v, &layout);

        rgui_render::build_scene_from_view(&v, &layout, &paint_fn, frame, Some(tr))
    });

    // 7. 加载 Rhai 事件脚本（占位）
    let rhai_path = std::path::PathBuf::from("handlers.rhai");
    if rhai_path.exists() {
        app.load_rhai_scripts(&[rhai_path.as_path()])?;
    }

    // 8. 启动事件循环
    app.run()
}

/// 从布局引擎获取 widget 的窗口绝对坐标 bounds（用于 hit test 交互区域注册）。
fn item_bounds(engine: &rgui_layout::LayoutEngine, id: rgui_core::id::WidgetId) -> Rect {
    let pos = engine
        .absolute_position(id)
        .unwrap_or(rgui_core::geometry::Point::new(0.0, 0.0));
    let size =
        engine
            .get_layout(id)
            .map_or(rgui_core::geometry::Size::new(100.0, 40.0), |cached| {
                rgui_core::geometry::Size::new(cached.result.size.width, cached.result.size.height)
            });
    Rect::new(pos.x, pos.y, size.width, size.height)
}

/// DFS 收集 AccordionItem 的 WidgetId（用于交互注册）。
fn collect_accordion_item_ids<M: AppMessage>(view: &WidgetView<M>) -> Vec<rgui_core::id::WidgetId> {
    let mut ids = Vec::new();
    collect_ids_recursive(view, &mut ids);
    ids
}

fn collect_ids_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    ids: &mut Vec<rgui_core::id::WidgetId>,
) {
    if view.widget_type == "WaAccordionItem" {
        if let Some(id) = view.id {
            ids.push(id);
        }
    }
    for child in &view.children {
        collect_ids_recursive(child, ids);
    }
}

/// AC07: 将 expanded 状态注入到 WidgetView 树的 AccordionItem props 中。
fn inject_expanded_props<M: AppMessage>(view: &mut WidgetView<M>, states: &[bool]) {
    let mut idx = 0;
    inject_recursive(view, states, &mut idx);
}

fn inject_recursive<M: AppMessage>(view: &mut WidgetView<M>, states: &[bool], idx: &mut usize) {
    if view.widget_type == "WaAccordionItem" {
        if *idx < states.len() {
            view.props.insert("expanded", PropValue::Bool(states[*idx]));
        }
        *idx += 1;
    }
    for child in &mut view.children {
        inject_recursive(child, states, idx);
    }
}
