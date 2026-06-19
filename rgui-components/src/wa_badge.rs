/// Translated from Web Awesome wa-badge
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-badge 组件状态。
///
/// 徽标组件，用于显示状态、计数或标签。无交互。
/// attention 动画（pulse/bounce）为 P2，当前保留字段但绘制时忽略。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaBadgeState {
    /// 主题变体：brand | neutral | success | warning | danger
    pub variant: String,
    /// 视觉外观：accent | filled | outlined | filled-outlined
    pub appearance: String,
    /// 是否 pill 形状（全圆角）
    pub pill: bool,
    /// 动效：none | pulse | bounce（P2 动画系统就绪前忽略）
    pub attention: String,
}

impl WaBadgeState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            variant: "brand".into(),
            appearance: "accent".into(),
            pill: false,
            attention: "none".into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaBadgeMessage {
    /// 占位消息——Badge 无交互事件
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaBadge;

impl WidgetSpec for WaBadge {
    type State = WaBadgeState;
    type Message = WaBadgeMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaBadge"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaBadge")
            .prop("variant", PropValue::str(state.variant.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("pill", PropValue::bool(state.pill))
            .prop("attention", PropValue::str(state.attention.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaBadgeMessage::NoOp => {},
        }
    }

    fn measure(&self, _state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // Badge 是叶子组件，最小尺寸约 1.25em × 1em。
        // 返回约束范围内的合理默认尺寸。
        let min_w = 20.0_f64.clamp(c.min_width, c.max_width);
        let min_h = 16.0_f64.clamp(c.min_height, c.max_height);
        let w = c.max_width.clamp(c.min_width, c.max_width).max(min_w);
        let h = c.max_height.clamp(c.min_height, c.max_height).max(min_h);
        Size::new(w, h)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let border_radius: f32 = if state.pill { 999.0 } else { 4.0 };
        let border_width: f64 = 1.0;

        // 根据 appearance 确定颜色（当前仅实现 brand 变体的基本颜色映射）
        let (bg_color, border_color, _text_color) = match state.appearance.as_str() {
            "outlined" => (
                Color::TRANSPARENT,
                // brand-border-loud
                Color::new(0.23, 0.49, 0.98, 1.0),
                // brand-on-quiet
                Color::new(0.12, 0.33, 0.76, 1.0),
            ),
            "filled" => (
                // brand-fill-normal
                Color::new(0.85, 0.90, 0.99, 1.0),
                Color::TRANSPARENT,
                // brand-on-normal
                Color::new(0.12, 0.33, 0.76, 1.0),
            ),
            "filled-outlined" => (
                // brand-fill-normal
                Color::new(0.85, 0.90, 0.99, 1.0),
                // brand-border-normal
                Color::new(0.67, 0.77, 0.96, 1.0),
                // brand-on-normal
                Color::new(0.12, 0.33, 0.76, 1.0),
            ),
            // "accent" (default) — brand-fill-loud background, brand-on-loud text
            _ => (
                // brand-fill-loud
                Color::new(0.12, 0.33, 0.76, 1.0),
                Color::TRANSPARENT,
                // brand-on-loud (white)
                Color::new(1.0, 1.0, 1.0, 1.0),
            ),
        };

        // 填充背景
        if bg_color != Color::TRANSPARENT {
            ctx.fill_rect(bounds, bg_color, border_radius);
        }

        // 绘制边框（仅 outline / filled-outlined 有可见边框）
        if border_color != Color::TRANSPARENT {
            // 简化的边框绘制：在内部画一个稍小的矩形
            let inner = Rect::new(
                bounds.origin.x + border_width,
                bounds.origin.y + border_width,
                (bounds.size.width - 2.0 * border_width).max(0.0),
                (bounds.size.height - 2.0 * border_width).max(0.0),
            );
            ctx.fill_rect(bounds, border_color, border_radius);
            if bg_color != Color::TRANSPARENT {
                ctx.fill_rect(inner, bg_color, border_radius.max(0.0).max(0.0_f32 - 1.0));
            }
        }
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("badge")
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
        assert_eq!(WaBadge.name(), "rgui_components::WaBadge");
    }

    #[test]
    fn view_has_variant_and_appearance() {
        let state = WaBadgeState::new();
        let v = WaBadge.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("variant"));
        assert!(v.props.contains_key("appearance"));
        assert!(v.props.contains_key("pill"));
    }

    #[test]
    fn view_pill_prop() {
        let mut state = WaBadgeState::new();
        state.pill = true;
        let v = WaBadge.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("pill").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b, "pill 应为 true"),
            _ => panic!("expected Bool prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaBadgeState::new();
        WaBadge.update(
            WaBadgeMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_has_minimum_size() {
        let state = WaBadgeState::new();
        let size = WaBadge.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 20.0, "宽度应 ≥ 20px，实际 {size:?}");
        assert!(size.height >= 16.0, "高度应 ≥ 16px，实际 {size:?}");
    }

    #[test]
    fn paint_accent_produces_ops() {
        let state = WaBadgeState::new(); // accent (default)
        let bounds = Rect::new(0.0, 0.0, 60.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaBadge.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "accent Badge 应产生绘制操作");
    }

    #[test]
    fn paint_outlined_produces_ops() {
        let mut state = WaBadgeState::new();
        state.appearance = "outlined".into();
        let bounds = Rect::new(0.0, 0.0, 60.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaBadge.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "outlined Badge 应产生绘制操作");
    }

    #[test]
    fn paint_pill_uses_large_radius() {
        let mut state = WaBadgeState::new();
        state.pill = true;
        let bounds = Rect::new(0.0, 0.0, 60.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaBadge.paint(&state, bounds, &mut ctx);
        // pill 模式下不 panic 即通过（border_radius = 999 不应导致绘制错误）
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaBadgeMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaBadgeState::schema_name(), "WaBadgeState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaBadgeState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaBadgeState>());
    }
}
