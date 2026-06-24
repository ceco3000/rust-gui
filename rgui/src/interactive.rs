//! 交互式组件初始化——为 .rgui 渲染路径的 WidgetSpec 组件自动注册交互处理器。
//!
//! AC10: 自动扫描 `.rgui` 节点中的 `onclick`/`on:toggle` 事件属性，
//! 调用 hit test 注册 + widget_instance handler 注册，
//! 使 main.rs 无需手写 Rust 桥接代码。

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
/// 自动注册 hit test 交互 + widget_instance handler。
/// 使 main.rs 无需手写 `app.register_interaction()` 桥接代码。
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
    let action = match action {
        Some(a) => a,
        None => return,
    };

    let size = layout
        .get_layout(widget_id)
        .map(|cached| cached.result.size)
        .unwrap_or(Size::ZERO);
    let abs_pos = layout.absolute_position(widget_id).unwrap_or(Point::ZERO);
    let rect = rgui_core::geometry::Rect::new(abs_pos.x, abs_pos.y, size.width, size.height);

    // 注册 hit test 交互
    app.register_interaction_with_chain(widget_id, rect, widget_chain.clone(), &action, |_| {});

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

    // AC10: 注册 widget_instance handler——点击时自动 toggle expanded 状态
    let store_handler = store.clone();
    app.register_widget_instance(widget_id, move |_action, _ctx| {
        if let Some(expanded) = store_handler.read::<bool>(widget_id) {
            store_handler.insert(widget_id, !expanded);
        }
        EventResult::Handled
    });
}
