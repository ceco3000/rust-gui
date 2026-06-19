/// Translated from Web Awesome wa-carousel-item
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::WidgetId;
use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::WidgetView;
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-carousel-item 组件状态。
///
/// CarouselItem 表示轮播中的单个幻灯片。WA 源无任何 @property 或 @event，
/// 仅渲染 `<slot></slot>` 包裹子内容。
///
/// Phase 0：纯容器，自身不绘制，子节点由 Carousel 布局驱动。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaCarouselItemState {
    // WA 源无 @property——空状态，仅作容器占位
}

impl WaCarouselItemState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

/// CarouselItem 无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaCarouselItemMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaCarouselItem;

impl WidgetSpec for WaCarouselItem {
    type State = WaCarouselItemState;
    type Message = WaCarouselItemMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaCarouselItem"
    }

    fn view(&self, _state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaCarouselItem")
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaCarouselItemMessage::NoOp => {},
        }
    }

    /// CarouselItem 是容器组件，尺寸由 Taffy 布局（Carousel 容器驱动）。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    /// 自身不绘制——仅作为语义容器包裹子节点。
    fn paint(&self, _state: &Self::State, _bounds: Rect, _ctx: &mut PaintContext) {
        // CarouselItem 自身无视觉绘制
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::new(WidgetId::from_u64(0), AccessibilityRole::Group, Rect::ZERO)
            .label("carousel item")
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::geometry::{BoxConstraints, Rect, Size};

    #[test]
    fn default_state() {
        let state = WaCarouselItemState::new();
        let _ = state; // 空状态，验证构造不 panic
    }

    #[test]
    fn message_noop() {
        let mut state = WaCarouselItemState::new();
        let mut ctx = UpdateContext::default();
        WaCarouselItem.update(WaCarouselItemMessage::NoOp, &mut state, &mut ctx);
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaCarouselItemState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaCarouselItem.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_produces_no_ops() {
        let state = WaCarouselItemState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarouselItem.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert_eq!(ops.len(), 0, "CarouselItem 自身不产生绘制操作");
    }

    #[test]
    fn accessibility_label() {
        let state = WaCarouselItemState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaCarouselItem.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("carousel item"));
    }

    #[test]
    fn view_contains_widget_type() {
        let state = WaCarouselItemState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaCarouselItem.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaCarouselItem");
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaCarouselItem.name(), "rgui_components::WaCarouselItem");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaCarouselItemMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaCarouselItemState::schema_name(), "WaCarouselItemState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaCarouselItemState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), std::any::TypeId::of::<WaCarouselItemState>());
    }
}
