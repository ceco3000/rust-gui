//! 交互式组件初始化——为 .rgui 渲染路径的 WidgetSpec 组件自动注册交互处理器。
//!
//! 当 `.rgui` 中声明了交互式组件（如 WaAccordionItem），框架需要：
//! 1. 在 WidgetStateStore 中初始化组件状态（从 WidgetView.props）
//! 2. 注册 widget_instance handler（Layer 1 事件处理）
//! 3. 注册交互区域（Layer 3 的 hit test 基础设施）
//!
//! ## 组件间协调（模式协调）
//!
//! 对于 WaAccordion 容器，当 `mode` 为 `single` 或 `single-collapsible` 时，
//! 展开一个 item 需自动折叠其他 items。本模块在遍历 WaAccordion 子节点时
//! 收集兄弟 WidgetId，注入 handler 实现模式协调逻辑。
//!
//! 本模块提供 `init_widget_instances()` 函数，在 `compute_view_layout`
//! 之后调用一次，即可让组件自主管理自己的交互行为。

use rgui_core::geometry::{Point, Rect, Size};
use rgui_core::id::WidgetId;
use rgui_core::traits::{AppMessage, EventResult, WidgetSpec};
use rgui_core::view::WidgetView;
use rgui_layout::LayoutEngine;

use crate::app::App;
use crate::widget_state::WidgetStateStore;

struct AccordionCtx {
    mode: String,
    sibling_ids: Vec<WidgetId>,
}

pub fn init_widget_instances<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    layout: &LayoutEngine,
) {
    let store = app.widget_state_store().clone();
    init_recursive(app, view, layout, &store, None);
}

fn init_recursive<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    layout: &LayoutEngine,
    store: &WidgetStateStore,
    accordion_ctx: Option<&AccordionCtx>,
) {
    let widget_id = match view.id {
        Some(id) => id,
        None => {
            for child in &view.children {
                init_recursive(app, child, layout, store, accordion_ctx);
            }
            return;
        }
    };

    match view.widget_type {
        "WaAccordion" => {
            let mode = get_str(&view.props, "mode").unwrap_or("multiple").to_string();
            let sibling_ids = collect_accordion_item_ids(&view.children);
            let ctx = AccordionCtx { mode, sibling_ids };
            for child in &view.children {
                init_recursive(app, child, layout, store, Some(&ctx));
            }
            return;
        }
        "WaAccordionItem" => {
            init_accordion_item(app, view, widget_id, layout, store, accordion_ctx);
        }
        _ => {}
    }

    register_onclick_if_present(app, view, widget_id, layout);

    for child in &view.children {
        init_recursive(app, child, layout, store, accordion_ctx);
    }
}

fn collect_accordion_item_ids<M: AppMessage>(children: &[WidgetView<M>]) -> Vec<WidgetId> {
    let mut ids = Vec::new();
    for child in children {
        if child.widget_type == "WaAccordionItem" {
            if let Some(id) = child.id {
                ids.push(id);
            }
        }
    }
    ids
}

fn init_accordion_item<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    widget_id: WidgetId,
    layout: &LayoutEngine,
    store: &WidgetStateStore,
    accordion_ctx: Option<&AccordionCtx>,
) {
    use rgui_components::wa_accordion_item::{
        WaAccordionItem, WaAccordionItemMessage, WaAccordionItemState,
    };

    let initial_state = {
        let mut s = WaAccordionItemState::new();
        if let Some(l) = get_str(&view.props, "label") {
            s.label = l.to_string();
        }
        if let Some(expanded) = view.props.get("expanded") {
            if let rgui_core::view::PropValue::Bool(b) = expanded {
                s.expanded = *b;
            }
        }
        if let Some(disabled) = view.props.get("disabled") {
            if let rgui_core::view::PropValue::Bool(b) = disabled {
                s.disabled = *b;
            }
        }
        if let Some(ip) = get_str(&view.props, "icon-placement") {
            s.icon_placement = ip.to_string();
        }
        if let Some(a) = get_str(&view.props, "appearance") {
            s.appearance = a.to_string();
        }
        if let Some(hl) = get_str(&view.props, "heading-level") {
            s.heading_level = hl.to_string();
        }
        if let Some(c) = get_str(&view.props, "content") {
            s.content = c.to_string();
        }
        s
    };
    store.insert(widget_id, initial_state);

    // 使用绝对坐标注册 hit test rect（LayoutEngine::absolute_position 累加祖先偏移）
    let size = layout
        .get_layout(widget_id)
        .map(|cached| cached.result.size)
        .unwrap_or(Size::ZERO);
    let abs_pos = layout.absolute_position(widget_id).unwrap_or(Point::ZERO);
    let rect = Rect::new(abs_pos.x, abs_pos.y, size.width, size.height);

    app.register_interaction(widget_id, rect, "toggle", |_| {});

    let store_clone = store.clone();
    let sibling_ids: Vec<WidgetId> = accordion_ctx
        .map(|ctx| ctx.sibling_ids.clone())
        .unwrap_or_default();
    let mode = accordion_ctx
        .map(|ctx| ctx.mode.clone())
        .unwrap_or_else(|| "multiple".to_string());

    app.register_widget_instance(widget_id, move |action, ctx| {
        if action == "toggle" {
            let mut should_collapse_others = false;
            store_clone.update::<WaAccordionItemState>(widget_id, |state| {
                WaAccordionItem.update(WaAccordionItemMessage::Trigger, state, ctx);
                should_collapse_others =
                    state.expanded && (mode == "single" || mode == "single-collapsible");
            });

            if should_collapse_others {
                for &sid in &sibling_ids {
                    if sid == widget_id {
                        continue;
                    }
                    store_clone.update::<WaAccordionItemState>(sid, |state| {
                        if state.expanded {
                            WaAccordionItem.update(WaAccordionItemMessage::Collapsed, state, ctx);
                        }
                    });
                }
            }

            EventResult::Handled
        } else {
            EventResult::Continue(String::new())
        }
    });
}

fn register_onclick_if_present<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    widget_id: WidgetId,
    layout: &LayoutEngine,
) {
    if let Some(rgui_core::view::PropValue::Str(action)) = view.props.get("onclick") {
        let size = layout
            .get_layout(widget_id)
            .map(|cached| cached.result.size)
            .unwrap_or(Size::ZERO);
        let abs_pos = layout.absolute_position(widget_id).unwrap_or(Point::ZERO);
        let rect = Rect::new(abs_pos.x, abs_pos.y, size.width, size.height);
        let action_owned = action.to_string();
        app.register_interaction(widget_id, rect, &action_owned, |_| {});
    }
}

fn get_str<'a>(
    props: &'a std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
    key: &str,
) -> Option<&'a str> {
    match props.get(key) {
        Some(rgui_core::view::PropValue::Str(s)) => Some(s),
        _ => None,
    }
}
