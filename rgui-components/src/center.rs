//! Center 组件——居中布局语法糖。
//!
//! 预设 `display: flex; align-items: center; justify-content: center`。
//! 仅接受单子节点。Center 无业务状态——纯布局。
//!
//! 在 `view()` 中映射为 `WidgetView.props` 中的 CSS 属性。
//!
//! ## 设计文档
//!
//! 源自 [D13 §3.5](../docs/D13-容器组件与布局组件设计.md#35-center居中布局语法糖)。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Center 业务状态（空——纯布局组件）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct CenterState {}

/// Center 消息类型（占位）。
///
/// Center 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum CenterMessage {
    NoOp,
}

/// Center 组件（unit struct）。
///
/// 居中布局语法糖——预设 `display: flex; align-items: center; justify-content: center`。
/// 实现 [`WidgetSpec`] trait。
pub struct Center;

impl WidgetSpec for Center {
    type State = CenterState;
    type Message = CenterMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Center"
    }

    fn view(&self, _s: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::Center");

        view = view.prop("display", PropValue::str("flex"));
        view = view.prop("align-items", PropValue::str("center"));
        view = view.prop("justify-content", PropValue::str("center"));

        view
    }

    fn update(&self, _msg: Self::Message, _state: &mut Self::State, _ctx: &mut UpdateContext) {}

    fn paint(&self, _state: &Self::State, _bounds: Rect, _ctx: &mut PaintContext) {}

    fn accessibility(&self, _state: &Self::State, _ctx: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name() {
        assert_eq!(Center.name(), "rgui_components::Center");
    }

    #[test]
    fn view_sets_display_flex() {
        let state = CenterState::default();
        let view = Center.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
    }

    #[test]
    fn view_sets_align_items_center() {
        let state = CenterState::default();
        let view = Center.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn view_sets_justify_content_center() {
        let state = CenterState::default();
        let view = Center.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn view_has_exactly_three_props() {
        let state = CenterState::default();
        let view = Center.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.len(), 3);
    }

    #[test]
    fn widget_type_is_center() {
        let state = CenterState::default();
        let view = Center.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.widget_type, "rgui_components::Center");
    }

    #[test]
    fn update_is_noop() {
        let mut state = CenterState::default();
        let mut ctx = UpdateContext::default();
        Center.update(CenterMessage::NoOp, &mut state, &mut ctx);
        // No state fields to verify — just ensure it doesn't panic
    }

    #[test]
    fn paint_is_empty() {
        let state = CenterState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Center.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = CenterState::default();
        let ctx = MeasureContext::default();
        let size = Center.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(CenterState::schema_name(), "CenterState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(CenterMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = CenterState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Center.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
