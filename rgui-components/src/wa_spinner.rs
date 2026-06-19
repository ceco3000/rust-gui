/// Translated from Web Awesome wa-spinner
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{Color, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-spinner 组件状态。
///
/// 加载动画指示器，阶段 0 用静态色块替代动画。
/// WA 源中无 `@property()` 声明，因此无状态字段。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaSpinnerState;

impl WaSpinnerState {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaSpinnerMessage {
    /// 占位消息——Spinner 无交互事件
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaSpinner;

impl WidgetSpec for WaSpinner {
    type State = WaSpinnerState;
    type Message = WaSpinnerMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaSpinner"
    }

    fn view(&self, _state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaSpinner")
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaSpinnerMessage::NoOp => {},
        }
    }

    fn measure(&self, _state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // Spinner 是叶子组件，默认 ~1em（24px）方形。
        // 返回约束范围内的合理默认尺寸。
        let default: f64 = 24.0;
        let w = default.clamp(c.min_width, c.max_width);
        let h = default.clamp(c.min_height, c.max_height);
        Size::new(w, h)
    }

    fn paint(&self, _state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 阶段 0：静态色块替代动画
        // 使用品牌色填充整个区域
        let color = Color::new(0.12, 0.33, 0.76, 1.0); // brand-fill-loud
        let border_radius: f32 = 4.0;
        ctx.fill_rect(bounds, color, border_radius);

        // 在中心绘制加载文字
        let font_size: f32 = (bounds.size.height * 0.5) as f32;
        let text = "↻";
        let text_width = font_size * 1.2; // 近似字符宽度
        let text_height = font_size;
        let text_bounds = Rect::new(
            bounds.origin.x + (bounds.size.width - text_width as f64) / 2.0,
            bounds.origin.y + (bounds.size.height - text_height as f64) / 2.0,
            text_width as f64,
            text_height as f64,
        );
        ctx.draw_text(text, text_bounds, Color::WHITE, font_size);
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("Loading")
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
        assert_eq!(WaSpinner.name(), "rgui_components::WaSpinner");
    }

    #[test]
    fn view_has_widget_type() {
        let state = WaSpinnerState::new();
        let v = WaSpinner.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaSpinner");
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaSpinnerState::new();
        WaSpinner.update(
            WaSpinnerMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_has_minimum_size() {
        let state = WaSpinnerState::new();
        let size = WaSpinner.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 24.0, "宽度应 ≥ 24px，实际 {size:?}");
        assert!(size.height >= 24.0, "高度应 ≥ 24px，实际 {size:?}");
    }

    #[test]
    fn measure_respects_max_constraints() {
        let state = WaSpinnerState::new();
        let size = WaSpinner.measure(
            &state,
            BoxConstraints::new(0.0, 16.0, 0.0, 16.0),
            &MeasureContext::default(),
        );
        assert!(
            size.width <= 16.0,
            "宽度应受 max 约束 ≤ 16px，实际 {size:?}"
        );
        assert!(
            size.height <= 16.0,
            "高度应受 max 约束 ≤ 16px，实际 {size:?}"
        );
    }

    #[test]
    fn paint_produces_ops() {
        let state = WaSpinnerState::new();
        let bounds = Rect::new(0.0, 0.0, 40.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaSpinner.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "Spinner 应产生绘制操作");
    }

    #[test]
    fn accessibility_is_loading() {
        let state = WaSpinnerState::new();
        let node = WaSpinner.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(
            node.label.as_deref(),
            Some("Loading"),
            "无障碍标签应为 Loading"
        );
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaSpinnerMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaSpinnerState::schema_name(), "WaSpinnerState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaSpinnerState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaSpinnerState>());
    }
}
