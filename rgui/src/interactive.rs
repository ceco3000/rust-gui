//! 交互式组件初始化——为 .rgui 渲染路径的 Tier 1/Tier 2 组件自动注册交互处理器。
//!
//! AC10: 自动扫描 `.rgui` 节点中的 `onclick`/`on:toggle` 事件属性，
//! 以及 Tier 2 组件标识（`_rhai_path`），自动注册 hit test + handler，
//! 使 main.rs 无需手写 Rust 桥接代码。

use rgui_core::geometry::{Point, Size};
use rgui_core::id::WidgetId;
use rgui_core::traits::AppMessage;
use rgui_core::view::{PropValue, WidgetView};
use rgui_layout::LayoutEngine;

use crate::app::App;
use rgui_core::coord_chain::CoordinateTransformChain;
use rgui_core::traits::EventResult;
use rgui_core::widget_state::WidgetStateStore;

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
/// 以及 Tier 2 组件（通过 `_rhai_path`），自动注册交互。
/// 组件专属的 mode 协调逻辑由组件目录（rgui-components）中的模块处理。
fn register_event_handlers<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    widget_id: WidgetId,
    layout: &LayoutEngine,
    widget_chain: &CoordinateTransformChain,
    store: &WidgetStateStore,
) {
    let onclick_action = view.props.get("onclick").and_then(|v| match v {
        PropValue::Str(s) => Some(s.to_string()),
        _ => None,
    });
    let ontoggle_action = view.props.get("on:toggle").and_then(|v| match v {
        PropValue::Str(s) => Some(s.to_string()),
        _ => None,
    });

    let action = ontoggle_action.or(onclick_action);

    // Tier 2 组件识别：通过 _rhai_path 字符串匹配
    let is_tier2 = view.props.get("_rhai_path")
        .and_then(|v| match v { PropValue::Str(s) => Some(s.as_ref().contains("accordionitem")), _ => None })
        .unwrap_or(false);

    if action.is_none() && !is_tier2 {
        return;
    }

    let size = layout.get_layout(widget_id).map(|c| c.result.size).unwrap_or(Size::ZERO);
    let abs_pos = layout.absolute_position(widget_id).unwrap_or(Point::ZERO);
    let rect = rgui_core::geometry::Rect::new(abs_pos.x, abs_pos.y, size.width, size.height);

    let action_str = action.unwrap_or_else(|| "toggle".to_string());
    app.register_interaction_with_chain(widget_id, rect, widget_chain.clone(), &action_str, |_| {});

    let initial_expanded = view.props.get("expanded")
        .and_then(|v| match v { PropValue::Bool(b) => Some(*b), _ => None })
        .unwrap_or(false);
    store.insert(widget_id, initial_expanded);

    // 基础 toggle handler——独立切换 expanded 状态。
    // 对于 Accordion，会被组件目录中的 mode 协调 handler 覆盖。
    let store_handler = store.clone();
    app.register_widget_instance(widget_id, move |_action, _ctx| {
        if let Some(expanded) = store_handler.read::<bool>(widget_id) {
            store_handler.insert(widget_id, !expanded);
        }
        EventResult::Handled
    });
}
