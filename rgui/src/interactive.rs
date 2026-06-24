//! 交互式组件初始化——为 .rgui 渲染路径的 Tier 1/Tier 2 组件自动注册交互处理器。
//!
//! AC10: 自动扫描 `.rgui` 节点中的 `onclick`/`on:toggle` 事件属性，
//! 以及 Tier 2 组件标识（`_rhai_path`），自动注册 hit test + handler，
//! 使 main.rs 无需手写 Rust 桥接代码。

use std::collections::HashMap;

use rgui_core::geometry::{Point, Size};
use rgui_core::id::WidgetId;
use rgui_core::traits::AppMessage;
use rgui_core::view::{PropValue, WidgetView};
use rgui_layout::LayoutEngine;

use crate::app::{App, CoordinateTransformChain};
use crate::widget_state::WidgetStateStore;
use rgui_core::traits::EventResult;

pub fn init_widget_instances<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    layout: &LayoutEngine,
) {
    let store = app.widget_state_store().clone();
    init_recursive(
        app,
        view,
        layout,
        &CoordinateTransformChain::default(),
        &store,
    );

    // AC09: 后处理 pass — 收集 Accordion 容器 + AccordionItem，注册 mode 协调
    let mut accordion_ctx = AccordionContext::default();
    collect_accordion_nodes(view, layout, &mut accordion_ctx);
    register_accordion_mode_coordination(app, &accordion_ctx);
}

fn init_recursive<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    layout: &LayoutEngine,
    parent_chain: &CoordinateTransformChain,
    store: &WidgetStateStore,
) {
    let widget_id = match view.id {
        Some(id) => id,
        None => {
            for child in &view.children {
                init_recursive(app, child, layout, parent_chain, store);
            }
            return;
        },
    };
    let widget_chain = layout
        .get_layout(widget_id)
        .map(|cached| parent_chain.translated(cached.result.position))
        .unwrap_or_else(|| parent_chain.clone());

    register_event_handlers(app, view, widget_id, layout, &widget_chain, store);

    for child in &view.children {
        init_recursive(app, child, layout, &widget_chain, store);
    }
}

/// AC10: 扫描 `.rgui` 节点的 `onclick`/`on:toggle` 事件属性，
/// 以及 Tier 2 AccordionItem（通过 `_rhai_path`），
/// 自动注册 hit test 交互 + widget_instance handler。
fn register_event_handlers<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    widget_id: WidgetId,
    layout: &LayoutEngine,
    widget_chain: &CoordinateTransformChain,
    store: &WidgetStateStore,
) {
    // 检查 onclick/on:toggle prop（通用）
    let onclick_action = view.props.get("onclick").and_then(|v| match v {
        PropValue::Str(s) => Some(s.to_string()),
        _ => None,
    });
    let ontoggle_action = view.props.get("on:toggle").and_then(|v| match v {
        PropValue::Str(s) => Some(s.to_string()),
        _ => None,
    });

    let action = ontoggle_action.or(onclick_action);

    // 检查是否为 Tier 2 AccordionItem（通过 _rhai_path）
    let is_tier2_accordion_item = view
        .props
        .get("_rhai_path")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref().contains("accordionitem")),
            _ => None,
        })
        .unwrap_or(false);

    if action.is_none() && !is_tier2_accordion_item {
        return;
    }

    let size = layout
        .get_layout(widget_id)
        .map(|cached| cached.result.size)
        .unwrap_or(Size::ZERO);
    let abs_pos = layout.absolute_position(widget_id).unwrap_or(Point::ZERO);
    let rect = rgui_core::geometry::Rect::new(abs_pos.x, abs_pos.y, size.width, size.height);

    // 注册 hit test 交互
    let action_str = action.unwrap_or_else(|| "toggle".to_string());
    app.register_interaction_with_chain(widget_id, rect, widget_chain.clone(), &action_str, |_| {});

    // 初始化 expanded 状态到 WidgetStateStore（从 .rgui props 读取初始值）
    let initial_expanded = view
        .props
        .get("expanded")
        .and_then(|v| match v {
            PropValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    store.insert(widget_id, initial_expanded);

    // 注册 widget_instance handler——点击时自动 toggle expanded 状态
    let store_handler = store.clone();
    app.register_widget_instance(widget_id, move |_action, _ctx| {
        if let Some(expanded) = store_handler.read::<bool>(widget_id) {
            store_handler.insert(widget_id, !expanded);
        }
        EventResult::Handled
    });
}

// ═══════════════════════════════════════════════════════════
// AC09: Accordion mode 协调（后处理 pass）
// ═══════════════════════════════════════════════════════════

#[derive(Default)]
struct AccordionContext {
    /// Accordion 容器 WidgetId → (mode, Vec<AccordionItem WidgetId>)
    containers: HashMap<WidgetId, (String, Vec<WidgetId>)>,
    /// item WidgetId → 父容器 WidgetId
    item_to_parent: HashMap<WidgetId, WidgetId>,
}

fn collect_accordion_nodes<M: AppMessage>(
    view: &WidgetView<M>,
    layout: &LayoutEngine,
    ctx: &mut AccordionContext,
) {
    collect_recursive(view, layout, ctx, None);
}

fn collect_recursive<M: AppMessage>(
    view: &WidgetView<M>,
    layout: &LayoutEngine,
    ctx: &mut AccordionContext,
    current_accordion: Option<WidgetId>,
) {
    let is_accordion = is_accordion_container(view);
    let is_item = is_accordion_item(view);

    if let Some(id) = view.id {
        if is_accordion {
            let mode = read_mode(view);
            ctx.containers
                .entry(id)
                .or_insert_with(|| (mode, Vec::new()));
        }
        if is_item {
            if let Some(acc_id) = current_accordion {
                ctx.item_to_parent.insert(id, acc_id);
                if let Some((_, items)) = ctx.containers.get_mut(&acc_id) {
                    items.push(id);
                }
            }
        }
    }

    let next_accordion = if is_accordion {
        view.id
    } else {
        current_accordion
    };

    for child in &view.children {
        collect_recursive(child, layout, ctx, next_accordion);
    }
}

fn register_accordion_mode_coordination(app: &mut App, ctx: &AccordionContext) {
    if ctx.containers.is_empty() || ctx.item_to_parent.is_empty() {
        return;
    }

    let store = app.widget_state_store().clone();

    for (&_container_id, (mode, item_ids)) in &ctx.containers {
        if item_ids.is_empty() {
            continue;
        }

        let mode = mode.clone();
        let item_ids = item_ids.clone();

        for &item_id in &item_ids {
            let sibling_ids = item_ids
                .iter()
                .filter(|&&id| id != item_id)
                .copied()
                .collect::<Vec<_>>();
            let s = store.clone();
            let m = mode.clone();

            app.register_widget_instance(item_id, move |_action, _ctx| {
                match m.as_str() {
                    "single" => {
                        let was_expanded = s.read::<bool>(item_id).unwrap_or(false);
                        if !was_expanded {
                            for &sid in &sibling_ids {
                                s.insert(sid, false);
                            }
                            s.insert(item_id, true);
                        }
                    }
                    "single-collapsible" => {
                        let was_expanded = s.read::<bool>(item_id).unwrap_or(false);
                        if was_expanded {
                            s.insert(item_id, false);
                        } else {
                            for &sid in &sibling_ids {
                                s.insert(sid, false);
                            }
                            s.insert(item_id, true);
                        }
                    }
                    _ => {
                        // multiple: independent toggle
                        let expanded = s.read::<bool>(item_id).unwrap_or(false);
                        s.insert(item_id, !expanded);
                    }
                }
                EventResult::Handled
            });
        }
    }
}

fn is_accordion_container<M: AppMessage>(view: &WidgetView<M>) -> bool {
    view.props
        .get("_rhai_path")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref().contains("accordion.rhai")),
            _ => None,
        })
        .unwrap_or(false)
}

fn is_accordion_item<M: AppMessage>(view: &WidgetView<M>) -> bool {
    view.props
        .get("_rhai_path")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref().contains("accordionitem")),
            _ => None,
        })
        .unwrap_or(false)
}

fn read_mode<M: AppMessage>(view: &WidgetView<M>) -> String {
    view.props
        .get("mode")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "multiple".to_string())
}
