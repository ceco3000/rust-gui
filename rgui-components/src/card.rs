//! Card 组件——装饰容器。
//!
//! Container 的子集，叠加视觉样式（background_color、border_radius、
//! border_width、border_color、box_shadow）。`paint()` 非空——
//! 绘制背景矩形、边框、阴影。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{
    AccessContext, MeasureContext, PaintContext, PaintOp, UpdateContext, ViewContext,
};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Card 业务状态。
///
/// 所有字段均为 `Option`——`None` 表示使用默认值。
/// Color 字段以 `(r, g, b, a)` 元组存储以支持 serde 序列化。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct CardState {
    /// 背景颜色（默认白色）。(r, g, b, a) 各分量 [0.0, 1.0]。
    pub background_color: Option<(f64, f64, f64, f64)>,
    /// 圆角半径（逻辑像素，默认 8.0）。
    pub border_radius: Option<f64>,
    /// 边框宽度（逻辑像素，默认 1.0）。
    pub border_width: Option<f64>,
    /// 边框颜色（默认浅灰）。(r, g, b, a) 各分量 [0.0, 1.0]。
    pub border_color: Option<(f64, f64, f64, f64)>,
    /// 阴影高度——阴影偏移量（默认 2.0）。
    pub elevation: Option<f64>,
}

impl CardState {
    fn background_color(&self) -> Color {
        self.background_color
            .map(|(r, g, b, a)| Color::new(r, g, b, a))
            .unwrap_or(Color::WHITE)
    }

    fn border_color(&self) -> Color {
        self.border_color
            .map(|(r, g, b, a)| Color::new(r, g, b, a))
            .unwrap_or(Color::rgb(0.85, 0.85, 0.85))
    }
}

/// Card 消息类型（占位）。
///
/// Card 本身不产生交互消息，提供此枚举以满足 `WidgetSpec` 关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum CardMessage {
    NoOp,
}

/// Card 组件（unit struct）。
///
/// 带视觉装饰的容器组件。实现 [`WidgetSpec`] trait。
pub struct Card;

impl Card {
    /// 返回默认圆角半径。
    fn default_radius() -> f64 {
        8.0
    }

    /// 返回默认边框宽度。
    fn default_border_w() -> f64 {
        1.0
    }

    /// 返回默认阴影高度。
    fn default_elevation() -> f64 {
        2.0
    }

    fn color_to_hex(c: Color) -> String {
        let arr = c.to_u8_array();
        format!("#{:02X}{:02X}{:02X}{:02X}", arr[0], arr[1], arr[2], arr[3])
    }
}

impl WidgetSpec for Card {
    type State = CardState;
    type Message = CardMessage;

    fn name(&self) -> &'static str {
        "rgui_components::Card"
    }

    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::new("rgui_components::Card");

        // 视觉 CSS 属性
        view = view.prop(
            "background-color",
            PropValue::str(Self::color_to_hex(s.background_color())),
        );

        let br = s.border_radius.unwrap_or_else(Self::default_radius);
        view = view.prop("border-radius", br);

        let bw = s.border_width.unwrap_or_else(Self::default_border_w);
        view = view.prop("border-width", bw);

        view = view.prop(
            "border-color",
            PropValue::str(Self::color_to_hex(s.border_color())),
        );

        let elev = s.elevation.unwrap_or_else(Self::default_elevation);
        view = view.prop(
            "box-shadow",
            PropValue::str(format!("{} {} {}", 2.0, 4.0, elev)),
        );

        view
    }

    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}

    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let bg = s.background_color();
        let br = s.border_radius.unwrap_or_else(Self::default_radius);
        let bw = s.border_width.unwrap_or_else(Self::default_border_w);
        let bc = s.border_color();
        let elev = s.elevation.unwrap_or_else(Self::default_elevation);

        let radius = br as f32;

        // 阴影（在最底层，偏移绘制）
        if elev > 0.0 {
            let shadow_color = Color::new(0.0, 0.0, 0.0, 0.15);
            let shadow_rect = Rect::new(
                bounds.origin.x + elev,
                bounds.origin.y + elev,
                bounds.size.width,
                bounds.size.height,
            );
            ctx.fill_rect(shadow_rect, shadow_color, radius);
        }

        // 背景
        ctx.fill_rect(bounds, bg, radius);

        // 边框（在最顶层，半透明描边效果）
        if bw > 0.0 {
            ctx.fill_rect(bounds, bc.with_alpha(0.3), radius);
        }
    }

    fn accessibility(&self, _: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name() {
        assert_eq!(Card.name(), "rgui_components::Card");
    }

    #[test]
    fn view_empty_state_has_default_visuals() {
        let state = CardState::default();
        let view = Card.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        // bg white, radius 8, border 1px light gray, shadow
        assert_eq!(
            view.props.get("background-color"),
            Some(&PropValue::str("#FFFFFFFF".to_string()))
        );
        assert_eq!(view.props.get("border-radius"), Some(&PropValue::from(8.0)));
        assert_eq!(view.props.get("border-width"), Some(&PropValue::from(1.0)));
        assert_eq!(
            view.props.get("border-color"),
            Some(&PropValue::str("#D9D9D9FF".to_string()))
        );
        assert!(view.props.contains_key("box-shadow"));
    }

    #[test]
    fn view_custom_background_color() {
        let state = CardState {
            background_color: Some((1.0, 0.0, 0.0, 1.0)),
            ..Default::default()
        };
        let view = Card.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("background-color"),
            Some(&PropValue::str("#FF0000FF".to_string()))
        );
    }

    #[test]
    fn view_custom_border_radius() {
        let state = CardState {
            border_radius: Some(4.0),
            ..Default::default()
        };
        let view = Card.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("border-radius"), Some(&PropValue::from(4.0)));
    }

    #[test]
    fn view_custom_border_width() {
        let state = CardState {
            border_width: Some(2.0),
            ..Default::default()
        };
        let view = Card.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(view.props.get("border-width"), Some(&PropValue::from(2.0)));
    }

    #[test]
    fn view_custom_border_color() {
        let state = CardState {
            border_color: Some((0.0, 0.0, 0.0, 1.0)),
            ..Default::default()
        };
        let view = Card.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            view.props.get("border-color"),
            Some(&PropValue::str("#000000FF".to_string()))
        );
    }

    #[test]
    fn view_custom_elevation() {
        let state = CardState {
            elevation: Some(4.0),
            ..Default::default()
        };
        let view = Card.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(view.props.contains_key("box-shadow"));
    }

    #[test]
    fn update_is_noop() {
        let mut state = CardState::default();
        let mut ctx = UpdateContext::default();
        Card.update(CardMessage::NoOp, &mut state, &mut ctx);
        assert!(state.background_color.is_none());
    }

    #[test]
    fn paint_with_defaults() {
        let state = CardState::default();
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        Card.paint(&state, bounds, &mut ctx);
        // shadow (with elevation=2) + background + border = 3 ops
        assert_eq!(ctx.op_count(), 3);
    }

    #[test]
    fn paint_no_elevation_skips_shadow() {
        let state = CardState {
            elevation: Some(0.0),
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        Card.paint(&state, bounds, &mut ctx);
        // only background + border = 2 ops
        assert_eq!(ctx.op_count(), 2);
    }

    #[test]
    fn paint_no_border_skips_border() {
        let state = CardState {
            border_width: Some(0.0),
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        Card.paint(&state, bounds, &mut ctx);
        // shadow + background = 2 ops
        assert_eq!(ctx.op_count(), 2);
    }

    #[test]
    fn paint_shadow_is_drawn_before_background() {
        let state = CardState {
            elevation: Some(3.0),
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        Card.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // First op should be shadow (offset)
        assert!(matches!(ops[0], PaintOp::FillRect { .. }));
        // Second op should be background (at bounds origin)
        assert!(matches!(ops[1], PaintOp::FillRect { .. }));
        // Third op should be border
        assert!(matches!(ops[2], PaintOp::FillRect { .. }));
    }

    #[test]
    fn measure_returns_zero() {
        let state = CardState::default();
        let ctx = MeasureContext::default();
        let size = Card.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(CardState::schema_name(), "CardState");
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(CardMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = CardState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        let node = Card.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
