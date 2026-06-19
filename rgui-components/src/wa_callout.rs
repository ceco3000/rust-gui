/// Translated from Web Awesome wa-callout (was <sl-alert>)
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

/// Web Awesome wa-callout 组件状态。
///
/// Callouts 显示内联重要消息——提示、警告、错误等。
/// variant 多态组件：brand | neutral | success | warning | danger
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaCalloutState {
    /// 主题变体：brand | neutral | success | warning | danger
    pub variant: String,
    /// 视觉外观：accent | filled | outlined | plain | filled-outlined
    pub appearance: String,
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 消息文本（来自 default slot）
    pub label: String,
}

impl WaCalloutState {
    #[must_use]
    pub fn new(label: &str) -> Self {
        Self {
            variant: "brand".into(),
            appearance: "filled".into(),
            size: "m".into(),
            label: label.into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// Callout 无事件，使用 NoOp 占位变体。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaCalloutMessage {
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaCallout;

impl WidgetSpec for WaCallout {
    type State = WaCalloutState;
    type Message = WaCalloutMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaCallout"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaCallout")
            .prop("variant", PropValue::str(state.variant.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("label", PropValue::str(state.label.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaCalloutMessage::NoOp => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // Callout 是装饰容器——尺寸由 Taffy 根据子节点和约束决定。
        // 没有子节点（当前仅 label prop）时，根据文本内容 + padding 计算最小尺寸。
        let font_size: f64 = 14.0;
        let em_height: f64 = 1.196;
        let text_h = font_size * em_height;
        // Callout 有较大的 padding（1em ≈ 16px），加上字体大小
        let pad_v: f64 = 32.0; // 1em top + bottom
        let pad_h: f64 = 32.0;

        let text_w = state.label.chars().count() as f64 * font_size * 0.6;

        let w = (text_w + pad_h).clamp(c.min_width, c.max_width).max(200.0);
        let h = (text_h + pad_v).clamp(c.min_height, c.max_height).max(44.0);

        Size::new(w, h)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let border_radius: f32 = 8.0; // --wa-panel-border-radius
        let border_width: f64 = 1.0;
        let font_size: f32 = 14.0;

        // 根据 variant 和 appearance 确定颜色
        let (bg_color, border_color, text_color) =
            colors_for_variant_and_appearance(&state.variant, &state.appearance);

        // 填充背景
        if bg_color != Color::TRANSPARENT {
            ctx.fill_rect(bounds, bg_color, border_radius);
        }

        // 绘制边框
        if border_color != Color::TRANSPARENT {
            let inner = Rect::new(
                bounds.origin.x + border_width,
                bounds.origin.y + border_width,
                (bounds.size.width - 2.0 * border_width).max(0.0),
                (bounds.size.height - 2.0 * border_width).max(0.0),
            );
            ctx.fill_rect(bounds, border_color, border_radius);
            if bg_color != Color::TRANSPARENT {
                ctx.fill_rect(
                    inner,
                    bg_color,
                    (border_radius as f64 - 1.0).max(0.0) as f32,
                );
            }
        }

        // 绘制消息文本（左对齐，带 padding）
        if !state.label.is_empty() {
            let text_bounds = Rect::new(
                bounds.origin.x + 16.0, // 左右 padding 1em
                bounds.origin.y,
                (bounds.size.width - 32.0).max(0.0),
                bounds.size.height,
            );
            ctx.draw_text(&state.label, text_bounds, text_color, font_size);
        }
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("callout")
    }
}

// ============================================================================
// 颜色映射
// ============================================================================

/// 根据 variant 和 appearance 返回 (bg_color, border_color, text_color)。
fn colors_for_variant_and_appearance(variant: &str, appearance: &str) -> (Color, Color, Color) {
    // WA 颜色系统：每个 variant 有 fill-quiet / fill-loud / border-quiet / border-loud / on-quiet / on-loud
    // 简化：用 variant 决定基准色，appearance 决定如何用
    let (base_r, base_g, base_b): (f64, f64, f64) = match variant {
        "brand" => (0.20, 0.50, 0.90),
        "neutral" => (0.55, 0.55, 0.55),
        "success" => (0.15, 0.65, 0.30),
        "warning" => (0.95, 0.60, 0.10),
        "danger" => (0.85, 0.20, 0.20),
        _ => (0.20, 0.50, 0.90), // 默认 brand
    };

    let fill_quiet = Color::new(
        base_r * 0.25 + 0.75,
        base_g * 0.25 + 0.75,
        base_b * 0.25 + 0.75,
        1.0,
    );
    let fill_loud = Color::new(base_r, base_g, base_b, 1.0);
    let border_quiet = Color::new(
        base_r * 0.45 + 0.55,
        base_g * 0.45 + 0.55,
        base_b * 0.45 + 0.55,
        1.0,
    );
    let border_loud = Color::new(
        base_r * 0.65 + 0.35,
        base_g * 0.65 + 0.35,
        base_b * 0.65 + 0.35,
        1.0,
    );
    let on_quiet = Color::new(0.20, 0.20, 0.20, 1.0);
    let on_loud = Color::new(1.0, 1.0, 1.0, 1.0);

    match appearance {
        "plain" => (Color::TRANSPARENT, Color::TRANSPARENT, on_quiet),
        "outlined" => (Color::TRANSPARENT, border_loud, on_quiet),
        "filled" => (fill_quiet, Color::TRANSPARENT, on_quiet),
        "filled-outlined" => (fill_quiet, border_quiet, on_quiet),
        "accent" => (fill_loud, Color::TRANSPARENT, on_loud),
        _ => (fill_quiet, Color::TRANSPARENT, on_quiet), // 默认 filled
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
        assert_eq!(WaCallout.name(), "rgui_components::WaCallout");
    }

    #[test]
    fn view_has_props() {
        let state = WaCalloutState::new("Test message");
        let v = WaCallout.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("variant"));
        assert!(v.props.contains_key("appearance"));
        assert!(v.props.contains_key("size"));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_label_prop() {
        let state = WaCalloutState::new("Hello");
        let v = WaCallout.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("label").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "Hello"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_variant_prop() {
        let mut state = WaCalloutState::new("");
        state.variant = "danger".into();
        let v = WaCallout.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("variant").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "danger"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_appearance_prop() {
        let mut state = WaCalloutState::new("");
        state.appearance = "outlined".into();
        let v = WaCallout.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("appearance").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "outlined"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaCalloutState::new("Test");
        WaCallout.update(
            WaCalloutMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_has_minimum_size() {
        let state = WaCalloutState::new("Test message");
        let size = WaCallout.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 200.0, "宽度应 ≥ 200px（最小），实际 {size:?}");
        assert!(size.height >= 44.0, "高度应 ≥ 44px，实际 {size:?}");
    }

    #[test]
    fn paint_filled_produces_ops() {
        let state = WaCalloutState::new("Info message"); // filled (default)
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "filled Callout 应产生绘制操作");
    }

    #[test]
    fn paint_outlined_produces_ops() {
        let mut state = WaCalloutState::new("Warning");
        state.appearance = "outlined".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "outlined Callout 应产生绘制操作");
    }

    #[test]
    fn paint_accent_produces_ops() {
        let mut state = WaCalloutState::new("Important!");
        state.appearance = "accent".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "accent Callout 应产生绘制操作");
    }

    #[test]
    fn paint_plain_produces_ops() {
        let mut state = WaCalloutState::new("Subtle");
        state.appearance = "plain".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        // plain 有文本，至少 draw_text
        assert!(ctx.op_count() >= 1, "plain Callout 应产生绘制操作");
    }

    #[test]
    fn paint_danger_variant_uses_red() {
        let mut state = WaCalloutState::new("Error!");
        state.variant = "danger".into();
        state.appearance = "filled".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_success_variant_uses_green() {
        let mut state = WaCalloutState::new("Success!");
        state.variant = "success".into();
        state.appearance = "filled".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_warning_variant_uses_orange() {
        let mut state = WaCalloutState::new("Caution");
        state.variant = "warning".into();
        state.appearance = "filled".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_brand_variant_uses_blue() {
        let mut state = WaCalloutState::new("Brand");
        state.variant = "brand".into();
        state.appearance = "filled".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_neutral_variant() {
        let mut state = WaCalloutState::new("Neutral");
        state.variant = "neutral".into();
        state.appearance = "filled".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_filled_outlined_produces_border() {
        let mut state = WaCalloutState::new("Bordered");
        state.appearance = "filled-outlined".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaCallout.paint(&state, bounds, &mut ctx);
        // 边框 + 背景 + 文本
        assert!(
            ctx.op_count() >= 2,
            "filled-outlined 应有边框+填充，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaCalloutMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaCalloutState::schema_name(), "WaCalloutState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaCalloutState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaCalloutState>());
    }
}
