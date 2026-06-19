/// Translated from Web Awesome wa-breadcrumb
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

/// Web Awesome wa-breadcrumb 组件状态。
///
/// 面包屑导航容器，在水平行中显示一组面包屑项，
/// 帮助用户理解当前位置并导航回父级页面。
/// 自身无视觉绘制，仅作为语义容器提供无障碍标签。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaBreadcrumbState {
    /// 无障碍标签（aria-label），屏幕阅读器会朗读此标签提供上下文。
    /// 不在屏幕上可见。
    pub label: String,
}

impl WaBreadcrumbState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaBreadcrumbMessage {
    /// 占位消息——Breadcrumb 无交互事件
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaBreadcrumb;

impl WidgetSpec for WaBreadcrumb {
    type State = WaBreadcrumbState;
    type Message = WaBreadcrumbMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaBreadcrumb"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaBreadcrumb")
            .prop("label", PropValue::str(state.label.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaBreadcrumbMessage::NoOp => {},
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        // Breadcrumb 是容器，尺寸由 Taffy 根据子节点和约束计算
        Size::ZERO
    }

    fn paint(&self, _state: &Self::State, _bounds: Rect, _ctx: &mut PaintContext) {
        // Breadcrumb 自身不绘制——它仅作为语义容器。
        // 视觉渲染由子节点（WaBreadcrumbItem）完成。
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label = if state.label.is_empty() {
            "breadcrumb"
        } else {
            state.label.as_str()
        };
        AccessibilityNode::none().label(label)
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
        assert_eq!(WaBreadcrumb.name(), "rgui_components::WaBreadcrumb");
    }

    #[test]
    fn view_has_label() {
        let mut state = WaBreadcrumbState::new();
        state.label = "Breadcrumb navigation".into();
        let v = WaBreadcrumb.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_default_empty_label() {
        let state = WaBreadcrumbState::new();
        let v = WaBreadcrumb.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("label") {
            Some(PropValue::Str(s)) => assert!(s.is_empty(), "默认 label 应为空字符串"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaBreadcrumbState::new();
        WaBreadcrumb.update(
            WaBreadcrumbMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_zero_delegating_to_layout() {
        let state = WaBreadcrumbState::new();
        let size = WaBreadcrumb.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Breadcrumb 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_produces_no_ops() {
        let state = WaBreadcrumbState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 30.0);
        let mut ctx = PaintContext::new(bounds);
        WaBreadcrumb.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "Breadcrumb 容器不产生绘制操作");
    }

    #[test]
    fn accessibility_default_label() {
        let state = WaBreadcrumbState::new();
        let node = WaBreadcrumb.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(
            node.label.as_deref(),
            Some("breadcrumb"),
            "默认 accessibility label 应为 'breadcrumb'"
        );
    }

    #[test]
    fn accessibility_custom_label() {
        let mut state = WaBreadcrumbState::new();
        state.label = "Navigation path".into();
        let node = WaBreadcrumb.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(
            node.label.as_deref(),
            Some("Navigation path"),
            "自定义 label 应被保留"
        );
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaBreadcrumbMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaBreadcrumbState::schema_name(), "WaBreadcrumbState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaBreadcrumbState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaBreadcrumbState>());
    }
}
