//! 交互式组件初始化——为 .rgui 渲染路径的 WidgetSpec 组件自动注册交互处理器。

use rgui_core::geometry::{Point, Size};
use rgui_core::id::WidgetId;
use rgui_core::traits::AppMessage;
use rgui_core::view::WidgetView;
use rgui_layout::LayoutEngine;

use crate::app::{App, CoordinateTransformChain};

pub fn init_widget_instances<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    layout: &LayoutEngine,
) {
    init_recursive(app, view, layout, &CoordinateTransformChain::default());
}

fn init_recursive<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    layout: &LayoutEngine,
    parent_chain: &CoordinateTransformChain,
) {
    let widget_id = match view.id {
        Some(id) => id,
        None => {
            for child in &view.children {
                init_recursive(app, child, layout, parent_chain);
            }
            return;
        }
    };
    let widget_chain = layout
        .get_layout(widget_id)
        .map(|cached| parent_chain.translated(cached.result.position))
        .unwrap_or_else(|| parent_chain.clone());

    register_onclick_if_present(app, view, widget_id, layout, &widget_chain);

    for child in &view.children {
        init_recursive(app, child, layout, &widget_chain);
    }
}

fn register_onclick_if_present<M: AppMessage>(
    app: &mut App,
    view: &WidgetView<M>,
    widget_id: WidgetId,
    layout: &LayoutEngine,
    widget_chain: &CoordinateTransformChain,
) {
    if let Some(rgui_core::view::PropValue::Str(action)) = view.props.get("onclick") {
        let size = layout
            .get_layout(widget_id)
            .map(|cached| cached.result.size)
            .unwrap_or(Size::ZERO);
        let abs_pos = layout.absolute_position(widget_id).unwrap_or(Point::ZERO);
        let rect = rgui_core::geometry::Rect::new(abs_pos.x, abs_pos.y, size.width, size.height);
        let action_owned = action.to_string();
        app.register_interaction_with_chain(
            widget_id,
            rect,
            widget_chain.clone(),
            &action_owned,
            |_| {},
        );
    }
}
