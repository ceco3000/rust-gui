/// Translated from Web Awesome wa-icon
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

/// Web Awesome wa-icon 组件状态。
///
/// 图标组件，表示可缩放矢量符号。当前实现使用 Unicode 字符渲染。
/// 跳过 family/variant/autoWidth/swapOpacity/src/library/rotate/flip/animation
/// 等 Web 专属或动画属性。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaIconState {
    /// 图标名称（如 "check", "chevron-down", "xmark" 等）
    pub name: String,
    /// 无障碍标签——为空时视为装饰性图标
    pub label: String,
    /// 尺寸：s | m | l
    pub size: String,
}

impl WaIconState {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: String::new(),
            size: "m".into(),
        }
    }
}

// ============================================================================
// Icon character mapping
// ============================================================================

/// 将图标名称映射到 Unicode 字符表示。
fn icon_char(name: &str) -> &'static str {
    match name {
        "check" => "\u{2713}",
        "chevron-down" => "\u{25BC}",
        "chevron-left" => "\u{25C0}",
        "chevron-right" => "\u{25B6}",
        "xmark" | "x-mark" => "\u{2717}",
        "circle" => "\u{25CF}",
        "plus" => "\u{002B}",
        "minus" => "\u{2212}",
        "star" => "\u{2605}",
        "ellipsis-vertical" => "\u{22EE}",
        "play" => "\u{25B6}",
        "pause" => "\u{23F8}",
        "user" => "\u{1F464}",
        "gear" => "\u{2699}",
        "file" => "\u{1F4C4}",
        "backward" => "\u{23EE}",
        "forward" => "\u{23ED}",
        "indeterminate" => "\u{2014}",
        "grip-vertical" => "\u{22EE}",
        "upload" => "\u{2B06}",
        "eye" => "\u{1F441}",
        "eye-slash" => "\u{1F576}",
        "copy" => "\u{1F4CB}",
        "clock" => "\u{1F552}",
        "calendar" => "\u{1F4C5}",
        "volume" => "\u{1F50A}",
        "volume-low" => "\u{1F509}",
        "volume-xmark" => "\u{1F507}",
        "picture-in-picture" => "\u{1F4FA}",
        "gauge" => "\u{1F3CE}",
        "backward-step" => "\u{23EE}",
        "forward-step" => "\u{23ED}",
        _ => "?",
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaIconMessage {
    /// 图标加载成功
    Load,
    /// 图标名未知或加载失败
    Error,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaIcon;

impl WidgetSpec for WaIcon {
    type State = WaIconState;
    type Message = WaIconMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaIcon"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaIcon")
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaIconMessage::Load => {},
            WaIconMessage::Error => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let icon_size: f64 = match state.size.as_str() {
            "s" => 12.0,
            "l" => 24.0,
            _ => 16.0, // "m" default
        };
        let w = icon_size.clamp(c.min_width, c.max_width);
        let h = icon_size.clamp(c.min_height, c.max_height);
        Size::new(w, h)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let icon_size: f64 = match state.size.as_str() {
            "s" => 12.0_f64,
            "l" => 24.0_f64,
            _ => 16.0_f64, // "m" default
        };

        let ch = icon_char(state.name.as_str());
        let color = Color::new(0.2, 0.2, 0.2, 1.0);

        // 在 bounds 中垂直居中绘制图标字符
        let font_size = icon_size as f32;
        let text_width = font_size * 0.6; // 估算单字符宽度
        let x = bounds.origin.x + (bounds.size.width - text_width as f64) / 2.0;
        let y = bounds.origin.y + (bounds.size.height - font_size as f64) / 2.0;

        let text_bounds = Rect::new(
            x,
            y,
            text_width as f64,
            font_size as f64,
        );
        ctx.draw_text(ch, text_bounds, color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        if state.label.is_empty() {
            // 装饰性图标——无标签
            AccessibilityNode::none()
        } else {
            AccessibilityNode::none().label(state.label.as_str())
        }
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
        assert_eq!(WaIcon.name(), "rgui_components::WaIcon");
    }

    #[test]
    fn view_has_name() {
        let state = WaIconState::new("check");
        let v = WaIcon.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("name"));
    }

    #[test]
    fn view_has_label_when_set() {
        let mut state = WaIconState::new("check");
        state.label = "Checkmark".into();
        let v = WaIcon.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("label").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "Checkmark"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_default_size_m() {
        let state = WaIconState::new("check");
        let v = WaIcon.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("size").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "m"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_load_is_handled() {
        let mut state = WaIconState::new("check");
        WaIcon.update(
            WaIconMessage::Load,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_error_is_handled() {
        let mut state = WaIconState::new("check");
        WaIcon.update(
            WaIconMessage::Error,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_size_s() {
        let state = {
            let mut s = WaIconState::new("check");
            s.size = "s".into();
            s
        };
        let size = WaIcon.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!((size.width - 12.0).abs() < 0.1, "size s 应为 12px，实际 {size:?}");
        assert!((size.height - 12.0).abs() < 0.1, "size s 高度应为 12px，实际 {size:?}");
    }

    #[test]
    fn measure_size_m() {
        let state = WaIconState::new("check");
        let size = WaIcon.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!((size.width - 16.0).abs() < 0.1, "size m 应为 16px，实际 {size:?}");
        assert!((size.height - 16.0).abs() < 0.1, "size m 高度应为 16px，实际 {size:?}");
    }

    #[test]
    fn measure_size_l() {
        let state = {
            let mut s = WaIconState::new("check");
            s.size = "l".into();
            s
        };
        let size = WaIcon.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!((size.width - 24.0).abs() < 0.1, "size l 应为 24px，实际 {size:?}");
        assert!((size.height - 24.0).abs() < 0.1, "size l 高度应为 24px，实际 {size:?}");
    }

    #[test]
    fn measure_clamped_by_constraints() {
        let state = WaIconState::new("check");
        let size = WaIcon.measure(
            &state,
            BoxConstraints::new(0.0, 10.0, 0.0, 10.0),
            &MeasureContext::default(),
        );
        assert!(size.width <= 10.0, "宽度应被约束限制，实际 {size:?}");
        assert!(size.height <= 10.0, "高度应被约束限制，实际 {size:?}");
    }

    #[test]
    fn paint_produces_ops() {
        let state = WaIconState::new("check");
        let bounds = Rect::new(0.0, 0.0, 16.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaIcon.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "图标绘制应产生绘制操作");
    }

    #[test]
    fn paint_unknown_name_shows_question() {
        let state = WaIconState::new("nonexistent-icon");
        let bounds = Rect::new(0.0, 0.0, 16.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaIcon.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "未知图标名也应产生绘制操作");
    }

    #[test]
    fn icon_char_known_names() {
        assert_eq!(icon_char("check"), "\u{2713}");
        assert_eq!(icon_char("chevron-down"), "\u{25BC}");
        assert_eq!(icon_char("xmark"), "\u{2717}");
        assert_eq!(icon_char("plus"), "\u{002B}");
    }

    #[test]
    fn icon_char_unknown_name() {
        assert_eq!(icon_char("nonexistent"), "?");
    }

    #[test]
    fn accessibility_decorative_when_no_label() {
        use rgui_core::geometry::Rect;
        let state = WaIconState::new("check");
        let node = WaIcon.accessibility(&state, &AccessContext::new(Rect::ZERO));
        // 装饰性图标不应有 label
        assert!(
            node.label.is_none() || node.label.as_ref().is_some_and(|s| s.is_empty()),
            "无 label 时不应有标签"
        );
    }

    #[test]
    fn accessibility_has_label_when_set() {
        use rgui_core::geometry::Rect;
        let mut state = WaIconState::new("check");
        state.label = "Checkmark".into();
        let node = WaIcon.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Checkmark"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaIconMessage::Load.message_name(), "load");
        assert_eq!(WaIconMessage::Error.message_name(), "error");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaIconState::schema_name(), "WaIconState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaIconState::new("check");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaIconState>());
    }
}
