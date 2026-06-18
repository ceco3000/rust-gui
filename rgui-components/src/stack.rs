//! Stack 组件——重叠布局。
//!
//! 所有子元素使用绝对定位，`z-index` 控制堆叠顺序。
//! 阶段 0 最小实现：所有子元素从 Stack 的 (0,0) 开始布局，
//! 手动坐标偏移 + children 顺序决定绘制层级。
//!
//! 在 `view()` 中映射为 `WidgetView.props` 中的 CSS 属性。
//!
//! ## 设计文档
//!
//! 源自 [D13 §4.2](../docs/D13-容器组件与布局组件设计.md#42-stack重叠布局)。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Stack 业务状态。
///
/// 所有字段均为 `Option`——`None` 表示该属性未被设置，使用默认值。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct StackState {
    /// 子元素对齐方式（"top-left" / "center" / "top-right" / "bottom-left" / "bottom-right"）。
    /// 默认 top-left。
    pub alignment: Option<String>,
}

/// Stack 消息类型（占位）。
///
/// Stack 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum StackMessage {
    NoOp,
}

/// Stack 组件（unit struct）。
///
/// 重叠布局容器——所有子元素使用绝对定位，`z-index` 通过 children 数组顺序决定。
/// 实现 [`WidgetSpec`] trait。
pub struct Stack;

impl WidgetSpec for Stack {
    type State = StackState;
    type Message = StackMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Stack"
    }

    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::Stack");

        // Stack is a positioned container — children use position: absolute
        view = view.prop("position", PropValue::str("relative"));
        view = view.prop("display", PropValue::str("flex"));

        // Map alignment to CSS flexbox properties
        match s.alignment.as_deref() {
            Some("center") => {
                view = view.prop("align-items", PropValue::str("center"));
                view = view.prop("justify-content", PropValue::str("center"));
            },
            Some("top-right") => {
                view = view.prop("align-items", PropValue::str("flex-end"));
            },
            Some("bottom-left") => {
                view = view.prop("justify-content", PropValue::str("flex-end"));
            },
            Some("bottom-right") => {
                view = view.prop("align-items", PropValue::str("flex-end"));
                view = view.prop("justify-content", PropValue::str("flex-end"));
            },
            // "top-left" is the default — no extra props needed
            _ => {},
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
        let stack = Stack;
        assert_eq!(stack.name(), "rgui_components::Stack");
    }

    #[test]
    fn view_default_state_sets_position_relative() {
        let state = StackState::default();
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("position"),
            Some(&PropValue::str("relative"))
        );
    }

    #[test]
    fn view_default_state_sets_display_flex() {
        let state = StackState::default();
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
    }

    #[test]
    fn view_with_alignment_top_left_has_no_align_overrides() {
        let state = StackState {
            alignment: Some("top-left".into()),
        };
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        // top-left is default: no extra align-items/justify-content props
        assert_eq!(view.props.get("align-items"), None);
        assert_eq!(view.props.get("justify-content"), None);
    }

    #[test]
    fn view_with_alignment_center_sets_center_props() {
        let state = StackState {
            alignment: Some("center".into()),
        };
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("center"))
        );
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn view_with_alignment_top_right() {
        let state = StackState {
            alignment: Some("top-right".into()),
        };
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("flex-end"))
        );
        assert_eq!(view.props.get("justify-content"), None);
    }

    #[test]
    fn view_with_alignment_bottom_left() {
        let state = StackState {
            alignment: Some("bottom-left".into()),
        };
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("align-items"), None);
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("flex-end"))
        );
    }

    #[test]
    fn view_with_alignment_bottom_right() {
        let state = StackState {
            alignment: Some("bottom-right".into()),
        };
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("flex-end"))
        );
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("flex-end"))
        );
    }

    #[test]
    fn view_widget_type_is_stack() {
        let state = StackState::default();
        let view = Stack.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.widget_type, "rgui_components::Stack");
    }

    #[test]
    fn update_is_noop() {
        let mut state = StackState::default();
        let mut ctx = UpdateContext::default();
        Stack.update(StackMessage::NoOp, &mut state, &mut ctx);
        // No state fields to verify — just ensure it doesn't panic
    }

    #[test]
    fn paint_is_empty() {
        let state = StackState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Stack.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = StackState::default();
        let ctx = MeasureContext::default();
        let size = Stack.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(StackState::schema_name(), "StackState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(StackMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = StackState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Stack.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
