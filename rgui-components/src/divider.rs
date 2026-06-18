//! Divider 组件——分隔线。
//!
//! 水平或垂直分隔线。零交互、零子节点。
//! `paint()` 通过 `fill_rect` 绘制一条线：
//! - 水平方向：宽度填满 bounds，高度 = thickness
//! - 垂直方向：高度填满 bounds，宽度 = thickness

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{
    AccessContext, MeasureContext, PaintContext, PaintOp, UpdateContext, ViewContext,
};
use rgui_core::geometry::{Axis, BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{Color, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Divider 业务状态。
///
/// - `direction`：Horizontal 或 Vertical，默认 Vertical
/// - `color`：线条颜色（默认浅灰）
/// - `thickness`：线宽（逻辑像素，默认 1.0 px）
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct DividerState {
    /// 方向（水平或垂直）。
    pub direction: Axis,
    /// 线条颜色（默认浅灰 `#D9D9D9`）。
    /// 以 (r, g, b, a) 元组存储以支持 serde 序列化。
    pub color: Option<(f64, f64, f64, f64)>,
    /// 线宽（逻辑像素，默认 1.0）。
    pub thickness: Option<f64>,
}

impl DividerState {
    fn color(&self) -> Color {
        self.color
            .map(|(r, g, b, a)| Color::new(r, g, b, a))
            .unwrap_or(Color::rgb(0.85, 0.85, 0.85))
    }

    fn thickness(&self) -> f64 {
        self.thickness.unwrap_or(1.0)
    }
}

/// Divider 消息类型（占位）。
///
/// Divider 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum DividerMessage {
    NoOp,
}

/// Divider 组件（unit struct）。
///
/// 水平或垂直分隔线。零子节点、零交互。实现 [`WidgetSpec`] trait。
pub struct Divider;

impl WidgetSpec for Divider {
    type State = DividerState;
    type Message = DividerMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Divider"
    }

    fn view(&self, _s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::Divider")
    }

    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}

    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let color = s.color();
        let thickness = s.thickness();

        match s.direction {
            Axis::Horizontal => {
                // 水平线：宽度填满 bounds，高度 = thickness
                // 居中放置（垂直方向）
                let line_y = bounds.origin.y + (bounds.size.height - thickness) / 2.0;
                let line_rect = Rect::new(bounds.origin.x, line_y, bounds.size.width, thickness);
                ctx.fill_rect(line_rect, color, 0.0);
            },
            Axis::Vertical => {
                // 垂直线：高度填满 bounds，宽度 = thickness
                // 居中放置（水平方向）
                let line_x = bounds.origin.x + (bounds.size.width - thickness) / 2.0;
                let line_rect = Rect::new(line_x, bounds.origin.y, thickness, bounds.size.height);
                ctx.fill_rect(line_rect, color, 0.0);
            },
        }
    }

    fn measure(
        &self,
        _s: &Self::State,
        _constraints: BoxConstraints,
        _ctx: &MeasureContext,
    ) -> Size {
        Size::ZERO
    }

    fn accessibility(&self, _: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- name ----

    #[test]
    fn name() {
        assert_eq!(Divider.name(), "rgui_components::Divider");
    }

    // ---- view ----

    #[test]
    fn view_returns_widget_view_with_component_name() {
        let state = DividerState::default();
        let view = Divider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.widget_type, "rgui_components::Divider");
    }

    // ---- update is noop ----

    #[test]
    fn update_is_noop() {
        let mut state = DividerState::default();
        let mut ctx = UpdateContext::default();
        Divider.update(DividerMessage::NoOp, &mut state, &mut ctx);
        // state unchanged
        assert_eq!(state.direction, Axis::Vertical);
        assert!(state.color.is_none());
        assert!(state.thickness.is_none());
    }

    // ---- paint: horizontal ----

    #[test]
    fn paint_horizontal_defaults() {
        let state = DividerState {
            direction: Axis::Horizontal,
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        Divider.paint(&state, bounds, &mut ctx);
        // One fill_rect operation
        assert_eq!(ctx.op_count(), 1);
    }

    #[test]
    fn paint_horizontal_line_centered_vertically() {
        let state = DividerState {
            direction: Axis::Horizontal,
            ..Default::default()
        };
        // bounds: 300×50, thickness default=1 → line_y = 0 + (50-1)/2 = 24.5
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        Divider.paint(&state, bounds, &mut ctx);

        let ops = ctx.into_operations();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PaintOp::FillRect { rect, radius, .. } => {
                // line fills bounds width
                assert_eq!(rect.origin.x, 0.0);
                assert_eq!(rect.size.width, 300.0);
                // line is centered vertically with thickness 1.0
                assert_eq!(rect.origin.y, 24.5);
                assert_eq!(rect.size.height, 1.0);
                // radius = 0 (no rounding)
                assert_eq!(*radius, 0.0);
            },
            _ => panic!("expected FillRect"),
        }
    }

    // ---- paint: vertical ----

    #[test]
    fn paint_vertical_defaults() {
        let state = DividerState {
            direction: Axis::Vertical,
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 50.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Divider.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 1);
    }

    #[test]
    fn paint_vertical_line_centered_horizontally() {
        let state = DividerState {
            direction: Axis::Vertical,
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 50.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        Divider.paint(&state, bounds, &mut ctx);

        let ops = ctx.into_operations();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PaintOp::FillRect { rect, radius, .. } => {
                // line centered horizontally with thickness 1.0
                assert_eq!(rect.origin.x, 24.5);
                assert_eq!(rect.size.width, 1.0);
                // line fills bounds height
                assert_eq!(rect.origin.y, 0.0);
                assert_eq!(rect.size.height, 300.0);
                assert_eq!(*radius, 0.0);
            },
            _ => panic!("expected FillRect"),
        }
    }

    // ---- paint: custom thickness ----

    #[test]
    fn paint_horizontal_custom_thickness() {
        let state = DividerState {
            direction: Axis::Horizontal,
            thickness: Some(3.0),
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        Divider.paint(&state, bounds, &mut ctx);

        let ops = ctx.into_operations();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PaintOp::FillRect { rect, .. } => {
                assert_eq!(rect.size.height, 3.0);
            },
            _ => panic!("expected FillRect"),
        }
    }

    // ---- measure ----

    #[test]
    fn measure_returns_zero() {
        let state = DividerState::default();
        let ctx = MeasureContext::default();
        let size = Divider.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    // ---- accessibility ----

    #[test]
    fn accessibility_returns_none() {
        let state = DividerState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Divider.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }

    // ---- state defaults ----

    #[test]
    fn state_default_direction_is_vertical() {
        let state = DividerState::default();
        assert_eq!(state.direction, Axis::Vertical);
    }

    #[test]
    fn state_default_color_is_light_gray() {
        let state = DividerState::default();
        let c = state.color();
        // RGB = 0.85
        assert!((c.r - 0.85).abs() < 0.001);
        assert!((c.g - 0.85).abs() < 0.001);
        assert!((c.b - 0.85).abs() < 0.001);
    }

    #[test]
    fn state_custom_color() {
        let state = DividerState {
            color: Some((1.0, 0.0, 0.0, 1.0)),
            ..Default::default()
        };
        let c = state.color();
        assert!((c.r - 1.0).abs() < 0.001);
        assert!((c.g - 0.0).abs() < 0.001);
        assert!((c.b - 0.0).abs() < 0.001);
    }

    #[test]
    fn state_default_thickness_is_1() {
        let state = DividerState::default();
        assert_eq!(state.thickness(), 1.0);
    }

    #[test]
    fn state_custom_thickness() {
        let state = DividerState {
            thickness: Some(2.0),
            ..Default::default()
        };
        assert_eq!(state.thickness(), 2.0);
    }

    // ---- derive ----

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(DividerState::schema_name(), "DividerState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(DividerMessage::NoOp.message_name(), "no_op");
    }
}
