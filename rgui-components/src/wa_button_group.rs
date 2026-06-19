/// Translated from Web Awesome wa-button-group
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-button-group 组件状态。
///
/// ButtonGroup 将多个 `<wa-button>` 组合为单一视觉单元，用于工具栏、
/// 分段控件等场景。它通过 `<slot>` 渲染子按钮，自身无视觉绘制。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaButtonGroupState {
    /// 辅助技术标签（如 `aria-label`），强烈推荐设置
    pub label: String,
    /// 方向：horizontal | vertical
    pub orientation: String,
}

impl WaButtonGroupState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            orientation: "horizontal".into(),
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// ButtonGroup 无自定义事件，NoOp 占位满足空枚举编译要求。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaButtonGroupMessage {
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaButtonGroup;

impl WidgetSpec for WaButtonGroup {
    type State = WaButtonGroupState;
    type Message = WaButtonGroupMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaButtonGroup"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaButtonGroup")
            .prop("orientation", PropValue::str(state.orientation.as_str()))
            .prop("label", PropValue::str(state.label.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaButtonGroupMessage::NoOp => {},
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        // ButtonGroup 是容器，尺寸由 Taffy 根据子节点和约束计算
        Size::ZERO
    }

    fn paint(&self, _state: &Self::State, _bounds: Rect, _ctx: &mut PaintContext) {
        // ButtonGroup 是纯容器，无自身视觉绘制。
        // 视觉样式（border-radius 移除、z-index 层叠）由子按钮样式系统处理。
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        use rgui_core::WidgetId;
        use rgui_core::a11y::AccessibilityRole;
        AccessibilityNode::new(WidgetId::from_u64(0), AccessibilityRole::Group, Rect::ZERO)
            .label(state.label.as_str())
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
        assert_eq!(WaButtonGroup.name(), "rgui_components::WaButtonGroup");
    }

    #[test]
    fn view_has_orientation() {
        let state = WaButtonGroupState::new();
        let v = WaButtonGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("orientation"));
    }

    #[test]
    fn view_default_horizontal() {
        let state = WaButtonGroupState::new();
        let v = WaButtonGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("orientation").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "horizontal"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_vertical_orientation() {
        let state = WaButtonGroupState {
            orientation: "vertical".into(),
            ..WaButtonGroupState::new()
        };
        let v = WaButtonGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("orientation").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "vertical"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_has_label() {
        let state = WaButtonGroupState {
            label: "Actions".into(),
            ..WaButtonGroupState::new()
        };
        let v = WaButtonGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn update_noop_does_not_panic() {
        let mut state = WaButtonGroupState::new();
        WaButtonGroup.update(
            WaButtonGroupMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_zero_delegating_to_layout() {
        let state = WaButtonGroupState::new();
        let size = WaButtonGroup.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "ButtonGroup 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_produces_no_ops() {
        let state = WaButtonGroupState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaButtonGroup.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "ButtonGroup 无自身视觉绘制");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaButtonGroupMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaButtonGroupState::schema_name(), "WaButtonGroupState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaButtonGroupState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaButtonGroupState>());
    }

    #[test]
    fn accessibility_has_label() {
        let state = WaButtonGroupState {
            label: "Actions".into(),
            ..WaButtonGroupState::new()
        };
        let node = WaButtonGroup.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Actions"));
    }

    #[test]
    fn accessibility_no_label_when_empty() {
        let state = WaButtonGroupState::new(); // label is ""
        let node = WaButtonGroup.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some(""));
    }
}
