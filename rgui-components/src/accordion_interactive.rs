//! Accordion 组件交互初始化——自动注册 Tier 2 Accordion/AccordionItem 的交互处理器。
//!
//! 利用框架 `InteractionHost` trait + `WidgetStateStore`，
//! 将 toggle + mode 协调收敛到组件层。
//!
//! AC09: mode 协调（single/single-collapsible/multiple）

use std::collections::HashMap;

use rgui_core::id::WidgetId;
use rgui_core::interaction::InteractionHost;
use rgui_core::traits::{AppMessage, EventResult};
use rgui_core::view::{PropValue, WidgetView};
use rgui_layout::LayoutEngine;

/// 为指定 WidgetView 树中的 Accordion 容器和 AccordionItem 叶子初始化交互。
///
/// 识别 Tier 2 展开后的节点（通过 `_rhai_path` prop），自动注册 toggle handler
/// 和 mode 协调逻辑。应在 `init_widget_instances` 之后调用。
pub fn init(
    app: &mut impl InteractionHost,
    view: &WidgetView<impl AppMessage>,
    layout: &LayoutEngine,
) {
    let mut ctx = AccordionContext::default();
    collect_accordion_nodes(view, layout, &mut ctx);
    register_mode_coordination(app, &ctx);
}

// ═══════════════════════════════════════
// 上下文收集
// ═══════════════════════════════════════

#[derive(Default)]
struct AccordionContext {
    containers: HashMap<WidgetId, (String, Vec<WidgetId>)>,
    item_to_parent: HashMap<WidgetId, WidgetId>,
}

fn collect_accordion_nodes<M: AppMessage>(
    view: &WidgetView<M>,
    _layout: &LayoutEngine,
    ctx: &mut AccordionContext,
) {
    collect_recursive(view, ctx, None);
}

fn collect_recursive<M: AppMessage>(
    view: &WidgetView<M>,
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

    let next = if is_accordion {
        view.id
    } else {
        current_accordion
    };
    for child in &view.children {
        collect_recursive(child, ctx, next);
    }
}

// ═══════════════════════════════════════
// mode 协调注册
// ═══════════════════════════════════════

fn register_mode_coordination(app: &mut impl InteractionHost, ctx: &AccordionContext) {
    if ctx.containers.is_empty() || ctx.item_to_parent.is_empty() {
        return;
    }

    let store = app.widget_state_store().clone();

    for (_container_id, (mode, item_ids)) in &ctx.containers {
        if item_ids.is_empty() {
            continue;
        }

        let mode = mode.clone();
        let item_ids = item_ids.clone();

        for &item_id in &item_ids {
            // 重写 handler——替换 register_event_handlers 注册的独立 toggle handler，
            // 实现 mode 协调（这是有意为之的覆盖行为）。
            let sibling_ids: Vec<WidgetId> = item_ids
                .iter()
                .filter(|&&id| id != item_id)
                .copied()
                .collect();
            let s = store.clone();
            let m = mode.clone();

            app.register_widget_instance(
                item_id,
                Box::new(move |_action, _ctx| {
                    match m.as_str() {
                        "single" => {
                            let was_expanded = s.read::<bool>(item_id).unwrap_or(false);
                            if !was_expanded {
                                for &sid in &sibling_ids {
                                    s.insert(sid, false);
                                }
                                s.insert(item_id, true);
                            }
                        },
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
                        },
                        _ => {
                            let expanded = s.read::<bool>(item_id).unwrap_or(false);
                            s.insert(item_id, !expanded);
                        },
                    }
                    EventResult::Handled
                }),
            );
        }
    }
}

// ═══════════════════════════════════════
// Tier 2 节点识别（通过 _rhai_path）
// ═══════════════════════════════════════

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

#[allow(dead_code)]
fn read_mode<M: AppMessage>(view: &WidgetView<M>) -> String {
    view.props
        .get("mode")
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "multiple".to_string())
}
