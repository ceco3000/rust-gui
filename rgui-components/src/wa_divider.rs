/// Translated from Web Awesome wa-divider
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-divider 组件状态。
///
/// 分隔线组件，用于在视觉上分隔或分组相邻元素。
/// 支持水平和垂直方向。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaDividerState {
    /// 方向：horizontal | vertical
    pub orientation: String,
}

impl WaDividerState {
    #[must_use]
    pub fn new(orientation: impl Into<String>) -> Self {
        Self {
            orientation: orientation.into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaDividerMessage {
    /// 占位消息——Divider 无交互事件
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaDivider;

impl WidgetSpec for WaDivider {
    type State = WaDividerState;
    type Message = WaDividerMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaDivider"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaDivider")
            .prop("orientation", PropValue::str(state.orientation.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaDividerMessage::NoOp => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        if state.orientation == "vertical" {
            // 垂直分隔线：窄宽度，拉伸可用高度
            let w = 2.0_f64.clamp(c.min_width, c.max_width);
            let h = c.max_height.clamp(c.min_height, c.max_height);
            Size::new(w, h)
        } else {
            // 水平分隔线：拉伸可用宽度，窄高度
            let w = c.max_width.clamp(c.min_width, c.max_width);
            let h = 2.0_f64.clamp(c.min_height, c.max_height);
            Size::new(w, h)
        }
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let line_color = Color::new(0.78, 0.78, 0.78, 1.0);

        if state.orientation == "vertical" {
            // 垂直分隔线：在水平中心画一条垂直窄线
            let line_width = 1.0;
            let x = bounds.origin.x + (bounds.size.width - line_width) / 2.0;
            let line_bounds = Rect::new(
                x.max(bounds.origin.x),
                bounds.origin.y,
                line_width.min(bounds.size.width),
                bounds.size.height,
            );
            ctx.fill_rect(line_bounds, line_color, 0.0);
        } else {
            // 水平分隔线：在垂直中心画一条水平窄线
            let line_height = 1.0;
            let y = bounds.origin.y + (bounds.size.height - line_height) / 2.0;
            let line_bounds = Rect::new(
                bounds.origin.x,
                y.max(bounds.origin.y),
                bounds.size.width,
                line_height.min(bounds.size.height),
            );
            ctx.fill_rect(line_bounds, line_color, 0.0);
        }
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("separator")
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    fn name() {
        assert_eq!(WaDivider.name(), "rgui_components::WaDivider");
    }

    #[test]
    fn view_has_orientation() {
        let state = WaDividerState::new("horizontal");
        let v = WaDivider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("orientation"));
    }

    #[test]
    fn view_vertical() {
        let state = WaDividerState::new("vertical");
        let v = WaDivider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("orientation").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "vertical"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaDividerState::new("horizontal");
        WaDivider.update(
            WaDividerMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_horizontal_stretches_width() {
        let state = WaDividerState::new("horizontal");
        let size = WaDivider.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(
            (size.width - 800.0).abs() < 0.1,
            "水平分隔线应拉伸到可用宽度，实际 {size:?}"
        );
        assert!(size.height <= 2.0, "高度应 ≤ 2px，实际 {size:?}");
    }

    #[test]
    fn measure_vertical_stretches_height() {
        let state = WaDividerState::new("vertical");
        let size = WaDivider.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width <= 2.0, "宽度应 ≤ 2px，实际 {size:?}");
        assert!(
            (size.height - 600.0).abs() < 0.1,
            "垂直分隔线应拉伸到可用高度，实际 {size:?}"
        );
    }

    #[test]
    fn paint_horizontal_produces_ops() {
        let state = WaDividerState::new("horizontal");
        let bounds = Rect::new(0.0, 0.0, 400.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaDivider.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "水平分隔线应产生绘制操作");
    }

    #[test]
    fn paint_vertical_produces_ops() {
        let state = WaDividerState::new("vertical");
        let bounds = Rect::new(0.0, 0.0, 20.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaDivider.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "垂直分隔线应产生绘制操作");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaDividerMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaDividerState::schema_name(), "WaDividerState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaDividerState::new("horizontal");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaDividerState>());
    }
}
