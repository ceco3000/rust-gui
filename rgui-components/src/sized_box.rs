//! SizedBox 组件——固定尺寸约束。
//!
//! 包裹单个子 widget，施加 `width` 和/或 `height` CSS 约束。
//! 如果某个维度为 `None`，则在该维度上不施加约束。
//!
//! 在 `view()` 中映射为 `WidgetView.props` 中的 CSS 属性。
//!
//! ## 设计文档
//!
//! 源自 [D13 §3.7](../docs/D13-容器组件与布局组件设计.md#37-sizedbox固定尺寸约束)。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// SizedBox 业务状态。
///
/// `width` 和 `height` 为可选固定尺寸（逻辑像素）。
/// 若某维度为 `None`，则不施加约束。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct SizedBoxState {
    /// 固定宽度（逻辑像素）。`None` 表示不约束。
    pub width: Option<f64>,
    /// 固定高度（逻辑像素）。`None` 表示不约束。
    pub height: Option<f64>,
}

/// SizedBox 消息类型（占位）。
///
/// SizedBox 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum SizedBoxMessage {
    NoOp,
}

/// SizedBox 组件（unit struct）。
///
/// 固定尺寸约束——包裹单个子 widget，施加 `width` 和/或 `height` CSS 约束。
/// 实现 [`WidgetSpec`] trait。
pub struct SizedBox;

impl WidgetSpec for SizedBox {
    type State = SizedBoxState;
    type Message = SizedBoxMessage;

    fn name(&self) -> &'static str {
        "rgui_components::SizedBox"
    }

    fn view(&self, s: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::SizedBox");

        if let Some(w) = s.width {
            view = view.prop("width", w);
        }
        if let Some(h) = s.height {
            view = view.prop("height", h);
        }

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
        assert_eq!(SizedBox.name(), "rgui_components::SizedBox");
    }

    #[test]
    fn view_default_state_has_no_props() {
        let state = SizedBoxState::default();
        let view = SizedBox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.len(), 0);
        assert_eq!(view.widget_type, "rgui_components::SizedBox");
    }

    #[test]
    fn view_sets_width_only() {
        let state = SizedBoxState {
            width: Some(200.0),
            height: None,
        };
        let view = SizedBox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("width"), Some(&PropValue::from(200.0)));
        assert_eq!(view.props.len(), 1);
    }

    #[test]
    fn view_sets_height_only() {
        let state = SizedBoxState {
            width: None,
            height: Some(100.0),
        };
        let view = SizedBox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("height"), Some(&PropValue::from(100.0)));
        assert_eq!(view.props.len(), 1);
    }

    #[test]
    fn view_sets_both_width_and_height() {
        let state = SizedBoxState {
            width: Some(200.0),
            height: Some(100.0),
        };
        let view = SizedBox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("width"), Some(&PropValue::from(200.0)));
        assert_eq!(view.props.get("height"), Some(&PropValue::from(100.0)));
        assert_eq!(view.props.len(), 2);
    }

    #[test]
    fn update_is_noop() {
        let mut state = SizedBoxState::default();
        let mut ctx = UpdateContext::default();
        SizedBox.update(SizedBoxMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.width, None);
        assert_eq!(state.height, None);
    }

    #[test]
    fn paint_is_empty() {
        let state = SizedBoxState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        SizedBox.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = SizedBoxState::default();
        let ctx = MeasureContext::default();
        let size = SizedBox.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(SizedBoxState::schema_name(), "SizedBoxState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(SizedBoxMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = SizedBoxState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = SizedBox.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
