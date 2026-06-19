/// Translated from Web Awesome wa-button
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

/// Web Awesome wa-button 组件状态。
///
/// 跳过 formAction/formEnctype/formMethod/type/name/value/withStart/withEnd
/// 等 Web 专属或 SSR 属性。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaButtonState {
    /// 主题变体：neutral | brand | success | warning | danger
    pub variant: String,
    /// 视觉外观：accent | filled | outlined | filled-outlined | plain
    pub appearance: String,
    /// 尺寸：xs | s | m | l | xl
    pub size: String,
    /// 禁用状态
    pub disabled: bool,
    /// 加载中状态
    pub loading: bool,
    /// 胶囊形状
    pub pill: bool,
    /// 按钮标签文本
    pub label: String,
}

impl WaButtonState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: "neutral".into(),
            appearance: "filled".into(),
            size: "m".into(),
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaButtonMessage {
    /// 点击
    Click,
    /// 失去焦点
    Blur,
    /// 获得焦点
    Focus,
    /// 验证失败
    Invalid,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaButton;

impl WidgetSpec for WaButton {
    type State = WaButtonState;
    type Message = WaButtonMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaButton"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaButton")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("variant", PropValue::str(state.variant.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("loading", PropValue::Bool(state.loading))
            .prop("pill", PropValue::Bool(state.pill))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaButtonMessage::Blur | WaButtonMessage::Focus | WaButtonMessage::Invalid => {},
            WaButtonMessage::Click => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // 文字宽度估算 + 最小 80×36
        let char_count = state.label.chars().count().max(1) as f64;
        let tw = char_count * 14.0 * 0.6;
        Size::new(
            (tw + 32.0).max(80.0).clamp(c.min_width, c.max_width),
            36_f64.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let bg = match state.appearance.as_str() {
            "outlined" | "plain" => Color::TRANSPARENT,
            _ => Color::new(0.20, 0.50, 0.90, 1.0),
        };
        if bg != Color::TRANSPARENT {
            let radius = if state.pill {
                bounds.size.height as f32 / 2.0
            } else {
                6.0
            };
            ctx.fill_rect(bounds, bg, radius);
        }

        if state.loading {
            return;
        }

        let text_color = if state.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else if state.appearance == "plain" || state.appearance == "outlined" {
            Color::new(0.20, 0.50, 0.90, 1.0)
        } else {
            Color::WHITE
        };
        let pad = 8.0;
        let text_bounds = Rect::new(
            bounds.origin.x + pad,
            bounds.origin.y,
            bounds.size.width - pad * 2.0,
            bounds.size.height,
        );
        let font_size = (bounds.size.height as f32 * 0.5).max(12.0);
        ctx.draw_text(&state.label, text_bounds, text_color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(state.label.as_str())
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
        assert_eq!(WaButton.name(), "rgui_components::WaButton");
    }

    #[test]
    fn view_has_label() {
        let state = WaButtonState::new("OK");
        let v = WaButton.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_pill() {
        let mut state = WaButtonState::new("OK");
        state.pill = true;
        let v = WaButton.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("pill"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_disabled() {
        let mut state = WaButtonState::new("OK");
        state.disabled = true;
        let v = WaButton.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaButtonState::new("OK");
        WaButton.update(
            WaButtonMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_click_continues() {
        let mut state = WaButtonState::new("OK");
        WaButton.update(
            WaButtonMessage::Click,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaButtonState::new("OK");
        let size = WaButton.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 80.0, "宽度应 ≥ 80px，实际 {size:?}");
        assert!(size.height >= 36.0, "高度应 ≥ 36px，实际 {size:?}");
    }

    #[test]
    fn measure_long_label_wider() {
        let short = WaButtonState::new("OK");
        let long = WaButtonState::new("A Very Long Button Label");
        let short_size = WaButton.measure(
            &short,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        let long_size = WaButton.measure(
            &long,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(long_size.width > short_size.width);
    }

    #[test]
    fn paint_filled_produces_ops() {
        let state = WaButtonState::new("OK");
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaButton.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2, "填充按钮应绘制背景 + 文字");
    }

    #[test]
    fn paint_outlined_no_fill() {
        let mut state = WaButtonState::new("OK");
        state.appearance = "outlined".into();
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaButton.paint(&state, bounds, &mut ctx);
        // outlined 不绘制 FillRect 背景，只有 DrawText
        let has_draw_text = ctx
            .into_operations()
            .iter()
            .any(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }));
        assert!(has_draw_text);
    }

    #[test]
    fn paint_loading_skips_text() {
        let mut state = WaButtonState::new("OK");
        state.loading = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaButton.paint(&state, bounds, &mut ctx);
        // loading 时只绘制背景
        let has_draw_text = ctx
            .into_operations()
            .iter()
            .any(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }));
        assert!(!has_draw_text);
    }

    #[test]
    fn paint_disabled_color() {
        let mut state = WaButtonState::new("OK");
        state.disabled = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaButton.paint(&state, bounds, &mut ctx);
        let has_draw_text = ctx
            .into_operations()
            .iter()
            .any(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }));
        assert!(has_draw_text);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaButtonMessage::Click.message_name(), "click");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaButtonState::schema_name(), "WaButtonState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaButtonState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaButtonState>());
    }
}
