//! Row 组件——水平布局语法糖。
//!
//! 预设 `display: flex; flex-direction: row`。
//! State 仅含 `main_axis_alignment`/`cross_axis_alignment`/`gap`，
//! 在 `view()` 中映射为 `WidgetView.props` 中的 CSS 属性。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Row 业务状态。
///
/// 所有字段均为 `Option`——`None` 表示该属性未被设置，
/// 布局引擎将使用默认值（`justify-content: flex-start`、
/// `align-items: stretch`、`gap: 0`）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct RowState {
    /// 主轴对齐方式（`justify-content`）。默认 `"flex-start"`。
    pub main_axis_alignment: Option<String>,
    /// 交叉轴对齐方式（`align-items`）。默认 `"stretch"`。
    pub cross_axis_alignment: Option<String>,
    /// 子元素间距（逻辑像素）。默认 `0.0`。
    pub gap: Option<f64>,
}

/// Row 消息类型（占位）。
///
/// Row 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum RowMessage {
    NoOp,
}

/// Row 组件（unit struct）。
///
/// 水平布局语法糖——预设 `display: flex; flex-direction: row`。
/// 实现 [`WidgetSpec`] trait。
pub struct Row;

impl WidgetSpec for Row {
    type State = RowState;
    type Message = RowMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Row"
    }

    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::Row");

        view = view.prop("display", PropValue::str("flex"));
        view = view.prop("flex-direction", PropValue::str("row"));

        if let Some(ref jc) = s.main_axis_alignment {
            view = view.prop("justify-content", PropValue::str(jc.as_str()));
        }
        if let Some(ref ai) = s.cross_axis_alignment {
            view = view.prop("align-items", PropValue::str(ai.as_str()));
        }
        if let Some(g) = s.gap {
            view = view.prop("gap", g);
        }

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
        assert_eq!(Row.name(), "rgui_components::Row");
    }

    #[test]
    fn view_empty_state_sets_display_flex_and_flex_direction_row() {
        let state = RowState::default();
        let view = Row.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        // Always sets display: flex
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
        // Always sets flex-direction: row
        assert_eq!(
            view.props.get("flex-direction"),
            Some(&PropValue::str("row"))
        );
    }

    #[test]
    fn view_empty_state_has_no_optional_props() {
        let state = RowState::default();
        let view = Row.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.len(), 2); // only display + flex-direction
    }

    #[test]
    fn view_sets_main_axis_alignment() {
        let state = RowState {
            main_axis_alignment: Some("center".into()),
            ..Default::default()
        };
        let view = Row.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn view_sets_cross_axis_alignment() {
        let state = RowState {
            cross_axis_alignment: Some("center".into()),
            ..Default::default()
        };
        let view = Row.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("center"))
        );
    }

    #[test]
    fn view_sets_gap() {
        let state = RowState {
            gap: Some(8.0),
            ..Default::default()
        };
        let view = Row.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("gap"), Some(&PropValue::from(8.0)));
    }

    #[test]
    fn view_sets_all_props() {
        let state = RowState {
            main_axis_alignment: Some("flex-end".into()),
            cross_axis_alignment: Some("baseline".into()),
            gap: Some(12.0),
        };
        let view = Row.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("display"), Some(&PropValue::str("flex")));
        assert_eq!(
            view.props.get("flex-direction"),
            Some(&PropValue::str("row"))
        );
        assert_eq!(
            view.props.get("justify-content"),
            Some(&PropValue::str("flex-end"))
        );
        assert_eq!(
            view.props.get("align-items"),
            Some(&PropValue::str("baseline"))
        );
        assert_eq!(view.props.get("gap"), Some(&PropValue::from(12.0)));
        assert_eq!(view.props.len(), 5);
    }

    #[test]
    fn update_is_noop() {
        let mut state = RowState::default();
        let mut ctx = UpdateContext::default();
        Row.update(RowMessage::NoOp, &mut state, &mut ctx);
        assert!(state.main_axis_alignment.is_none());
        assert!(state.cross_axis_alignment.is_none());
        assert!(state.gap.is_none());
    }

    #[test]
    fn paint_is_empty() {
        let state = RowState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Row.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = RowState::default();
        let ctx = MeasureContext::default();
        let size = Row.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(RowState::schema_name(), "RowState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(RowMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = RowState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Row.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
