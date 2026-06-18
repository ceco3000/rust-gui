//! Padding 组件——内边距语法糖。
//!
//! 预设 `padding` CSS 属性；仅接受单子节点。
//! 在 `view()` 中映射为 `WidgetView.props` 中的 CSS 属性。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Padding 业务状态。
///
/// `padding` 字段为四边相同的内边距（逻辑像素），默认 `0.0`。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct PaddingState {
    /// 内边距（四边相同，逻辑像素）。默认 `0.0`。
    pub padding: f64,
}

/// Padding 消息类型（占位）。
///
/// Padding 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum PaddingMessage {
    NoOp,
}

/// Padding 组件（unit struct）。
///
/// 内边距语法糖——预设 `display: flex` + `padding` CSS 属性，仅接受单子节点。
/// 实现 [`WidgetSpec`] trait。
pub struct Padding;

impl WidgetSpec for Padding {
    type State = PaddingState;
    type Message = PaddingMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Padding"
    }

    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::Padding");

        view = view.prop("display", PropValue::str("flex"));
        view = view.prop("padding", s.padding);

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
        assert_eq!(Padding.name(), "rgui_components::Padding");
    }

    #[test]
    fn view_default_state_sets_display_flex_and_zero_padding() {
        let state = PaddingState::default();
        let view = Padding.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        // Always sets display: flex
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
        // Always sets padding (even if zero)
        assert_eq!(view.props.get("padding"), Some(&PropValue::from(0.0)));
        assert_eq!(view.widget_type, "rgui_components::Padding");
    }

    #[test]
    fn view_sets_padding() {
        let state = PaddingState { padding: 16.0 };
        let view = Padding.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
        assert_eq!(view.props.get("padding"), Some(&PropValue::from(16.0)));
        assert_eq!(view.props.len(), 2);
    }

    #[test]
    fn view_sets_custom_padding() {
        let state = PaddingState { padding: 24.0 };
        let view = Padding.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("padding"), Some(&PropValue::from(24.0)));
    }

    #[test]
    fn update_is_noop() {
        let mut state = PaddingState::default();
        let mut ctx = UpdateContext::default();
        Padding.update(PaddingMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.padding, 0.0);
    }

    #[test]
    fn paint_is_empty() {
        let state = PaddingState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Padding.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = PaddingState::default();
        let ctx = MeasureContext::default();
        let size = Padding.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(PaddingState::schema_name(), "PaddingState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(PaddingMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = PaddingState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Padding.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
