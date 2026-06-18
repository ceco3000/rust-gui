//! Expanded 组件——弹性扩展包装器。
//!
//! 包裹单个子 widget，设置 `flex-grow` CSS 属性，使其在父 flex 容器中占据剩余空间。
//! **不是 Container**——不创建新的 flex 上下文，而是修改自身在父容器中的布局行为。
//!
//! 在 `view()` 中映射为 `WidgetView.props` 中的 CSS 属性。
//!
//! ## 设计文档
//!
//! 源自 [D13 §3.6](../docs/D13-容器组件与布局组件设计.md#36-expanded弹性扩展子节点)。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Expanded 业务状态。
///
/// `flex` 字段为 flex-grow 值（默认 `1.0`）。
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct ExpandedState {
    /// flex-grow 值。默认 `1.0`。
    pub flex: f64,
}

impl Default for ExpandedState {
    fn default() -> Self {
        Self { flex: 1.0 }
    }
}

/// Expanded 消息类型（占位）。
///
/// Expanded 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum ExpandedMessage {
    NoOp,
}

/// Expanded 组件（unit struct）。
///
/// 弹性扩展包装器——包裹单个子 widget，设置 `flex-grow` CSS 属性。
/// 实现 [`WidgetSpec`] trait。
pub struct Expanded;

impl WidgetSpec for Expanded {
    type State = ExpandedState;
    type Message = ExpandedMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Expanded"
    }

    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::Expanded");

        view = view.prop("flex-grow", s.flex);

        view
    }

    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}

    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}

    fn accessibility(&self, _: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name() {
        assert_eq!(Expanded.name(), "rgui_components::Expanded");
    }

    #[test]
    fn view_default_state_sets_flex_grow_to_one() {
        let state = ExpandedState::default();
        let view = Expanded.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("flex-grow"), Some(&PropValue::from(1.0)));
        assert_eq!(view.widget_type, "rgui_components::Expanded");
    }

    #[test]
    fn view_default_state_has_exactly_one_prop() {
        let state = ExpandedState::default();
        let view = Expanded.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.len(), 1);
    }

    #[test]
    fn view_sets_custom_flex_grow() {
        let state = ExpandedState { flex: 2.0 };
        let view = Expanded.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("flex-grow"), Some(&PropValue::from(2.0)));
    }

    #[test]
    fn view_sets_flex_grow_zero() {
        let state = ExpandedState { flex: 0.0 };
        let view = Expanded.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("flex-grow"), Some(&PropValue::from(0.0)));
    }

    #[test]
    fn update_is_noop() {
        let mut state = ExpandedState::default();
        let mut ctx = UpdateContext::default();
        Expanded.update(ExpandedMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.flex, 1.0);
    }

    #[test]
    fn paint_is_empty() {
        let state = ExpandedState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Expanded.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = ExpandedState::default();
        let ctx = MeasureContext::default();
        let size = Expanded.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(ExpandedState::schema_name(), "ExpandedState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(ExpandedMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = ExpandedState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Expanded.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
