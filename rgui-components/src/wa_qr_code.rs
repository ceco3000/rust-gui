/// Translated from Web Awesome wa-qr-code
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

/// Web Awesome wa-qr-code 组件状态。
///
/// 二维码组件，将 URL 或短文本编码为可扫描的二维码图像。
/// 阶段 0：渲染占位方形边框 + 文本标签（QR 码生成待后续实现）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaQrCodeState {
    /// 二维码的值（URL 或文本）
    pub value: String,
    /// 辅助设备朗读的标签。未指定时使用 value
    pub label: String,
    /// 二维码尺寸（像素）
    pub size: f64,
    /// 模块圆角半径（0.0 ~ 0.5）
    pub radius: f64,
    /// 纠错级别：L | M | Q | H
    pub error_correction: String,
}

impl WaQrCodeState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: String::new(),
            label: String::new(),
            size: 128.0,
            radius: 0.0,
            error_correction: "H".into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// QrCode 无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaQrCodeMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaQrCode;

impl WidgetSpec for WaQrCode {
    type State = WaQrCodeState;
    type Message = WaQrCodeMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaQrCode"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaQrCode")
            .prop("value", PropValue::str(state.value.as_str()))
            .prop("label", PropValue::str(state.label.as_str()))
            .prop(
                "size",
                PropValue::Float(ordered_float::OrderedFloat(state.size)),
            )
            .prop(
                "radius",
                PropValue::Float(ordered_float::OrderedFloat(state.radius)),
            )
            .prop(
                "error-correction",
                PropValue::str(state.error_correction.as_str()),
            )
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaQrCodeMessage::NoOp => {},
        }
    }

    /// QR 码尺寸由 size prop 决定，返回方形。
    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let s = state.size.clamp(c.min_width, c.max_width);
        let s = s.clamp(c.min_height, c.max_height);
        let s = s.clamp(0.0, c.max_width.min(c.max_height));
        Size::new(s, s)
    }

    /// 阶段 0：渲染白色方形背景 + 黑色边框 + 居中文本标签。
    ///
    /// 后续阶段将替换为实际 QR 码模块绘制（需引入 QR 生成库）。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let s = bounds.size.width.min(bounds.size.height);

        // 白色背景
        let bg_color = Color::new(1.0, 1.0, 1.0, 1.0);
        let border_radius: f32 = (state.radius * s) as f32;
        let square_rect = Rect::new(bounds.origin.x, bounds.origin.y, s, s);
        ctx.fill_rect(square_rect, bg_color, border_radius);

        // 黑色边框（宽 2px）
        let border_color = Color::new(0.0, 0.0, 0.0, 1.0);
        let border_width: f64 = 2.0;
        // 顶边
        ctx.fill_rect(
            Rect::new(square_rect.origin.x, square_rect.origin.y, s, border_width),
            border_color,
            0.0,
        );
        // 底边
        ctx.fill_rect(
            Rect::new(
                square_rect.origin.x,
                square_rect.origin.y + s - border_width,
                s,
                border_width,
            ),
            border_color,
            0.0,
        );
        // 左边
        ctx.fill_rect(
            Rect::new(square_rect.origin.x, square_rect.origin.y, border_width, s),
            border_color,
            0.0,
        );
        // 右边
        ctx.fill_rect(
            Rect::new(
                square_rect.origin.x + s - border_width,
                square_rect.origin.y,
                border_width,
                s,
            ),
            border_color,
            0.0,
        );

        // 居中文本标签
        let display_text = if state.label.is_empty() {
            if state.value.is_empty() {
                "QR Code"
            } else {
                state.value.as_str()
            }
        } else {
            state.label.as_str()
        };

        let font_size: f32 = f64::max(s * 0.10, 8.0) as f32;
        let text_color = Color::new(0.0, 0.0, 0.0, 1.0);
        // 将文本绘制在方形中心
        let text_bounds = square_rect;
        ctx.draw_text(display_text, text_bounds, text_color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label_text: &str = if state.label.is_empty() {
            if state.value.is_empty() {
                "QR Code"
            } else {
                state.value.as_str()
            }
        } else {
            state.label.as_str()
        };
        AccessibilityNode::none().label(label_text)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let state = WaQrCodeState::new();
        assert_eq!(state.value, "");
        assert_eq!(state.label, "");
        assert!((state.size - 128.0).abs() < 0.01);
        assert!((state.radius - 0.0).abs() < 0.01);
        assert_eq!(state.error_correction, "H");
    }

    #[test]
    fn state_with_value() {
        let state = WaQrCodeState {
            value: "https://example.com".into(),
            ..WaQrCodeState::new()
        };
        assert_eq!(state.value, "https://example.com");
    }

    #[test]
    fn state_with_custom_size() {
        let state = WaQrCodeState {
            size: 256.0,
            ..WaQrCodeState::new()
        };
        assert!((state.size - 256.0).abs() < 0.01);
    }

    #[test]
    fn state_with_error_correction() {
        let state = WaQrCodeState {
            error_correction: "L".into(),
            ..WaQrCodeState::new()
        };
        assert_eq!(state.error_correction, "L");
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaQrCode.name(), "rgui_components::WaQrCode");
    }

    #[test]
    fn message_noop() {
        let mut state = WaQrCodeState::new();
        let mut ctx = UpdateContext::default();
        WaQrCode.update(WaQrCodeMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.value, "");
    }

    #[test]
    fn measure_returns_square() {
        let state = WaQrCodeState::new(); // size=128
        let constraints = BoxConstraints::new(0.0, 500.0, 0.0, 500.0);
        let ctx = MeasureContext::default();
        let size = WaQrCode.measure(&state, constraints, &ctx);
        assert!((size.width - 128.0).abs() < 0.01);
        assert!((size.height - 128.0).abs() < 0.01);
    }

    #[test]
    fn measure_clamped_by_constraints() {
        let state = WaQrCodeState {
            size: 500.0,
            ..WaQrCodeState::new()
        };
        let constraints = BoxConstraints::new(0.0, 200.0, 0.0, 200.0);
        let ctx = MeasureContext::default();
        let size = WaQrCode.measure(&state, constraints, &ctx);
        assert!(size.width <= 200.0, "宽度应 ≤ 200，实际 {size:?}");
        assert!(size.height <= 200.0, "高度应 ≤ 200，实际 {size:?}");
    }

    #[test]
    fn measure_zero_size_returns_zero() {
        let state = WaQrCodeState::new(); // size=128
        let constraints = BoxConstraints::new(0.0, 0.0, 0.0, 0.0);
        let ctx = MeasureContext::default();
        let size = WaQrCode.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_produces_ops() {
        let state = WaQrCodeState {
            value: "test".into(),
            ..WaQrCodeState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 128.0, 128.0);
        let mut ctx = PaintContext::new(bounds);
        WaQrCode.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(
            ops.len() >= 2,
            "应产生至少 2 个绘制操作（背景+边框+文本），实际 {}",
            ops.len()
        );
    }

    #[test]
    fn paint_zero_bounds_no_ops() {
        let state = WaQrCodeState::new();
        let bounds = Rect::ZERO;
        let mut ctx = PaintContext::new(bounds);
        WaQrCode.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "零尺寸不应产生绘制操作");
    }

    #[test]
    fn view_has_props() {
        let state = WaQrCodeState {
            value: "https://example.com".into(),
            label: "Scan me".into(),
            ..WaQrCodeState::new()
        };
        let v = WaQrCode.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaQrCode");
        assert!(v.props.contains_key("value"));
        assert!(v.props.contains_key("label"));
        assert!(v.props.contains_key("size"));
        assert!(v.props.contains_key("radius"));
        assert!(v.props.contains_key("error-correction"));
    }

    #[test]
    fn view_value_prop() {
        let mut state = WaQrCodeState::new();
        state.value = "hello".into();
        let v = WaQrCode.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("value").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "hello"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_size_prop() {
        let mut state = WaQrCodeState::new();
        state.size = 256.0;
        let v = WaQrCode.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("size").unwrap() {
            PropValue::Float(f) => assert!((f.0 - 256.0).abs() < 0.01),
            _ => panic!("expected Float prop"),
        }
    }

    #[test]
    fn accessibility_uses_label_when_present() {
        let state = WaQrCodeState {
            value: "data".into(),
            label: "Scan QR".into(),
            ..WaQrCodeState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaQrCode.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("Scan QR"));
    }

    #[test]
    fn accessibility_falls_back_to_value() {
        let state = WaQrCodeState {
            value: "https://example.com".into(),
            label: String::new(),
            ..WaQrCodeState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaQrCode.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn accessibility_empty_value_uses_default() {
        let state = WaQrCodeState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaQrCode.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("QR Code"));
    }

    #[test]
    fn paint_with_label_prefers_label() {
        let state = WaQrCodeState {
            value: "data".into(),
            label: "My QR".into(),
            size: 100.0,
            ..WaQrCodeState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaQrCode.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 应该有背景 + 4 条边框 + 文本，至少 6 个操作
        assert!(ops.len() >= 6, "应有背景+4边框+文本，实际 {}", ops.len());
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaQrCodeMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaQrCodeState::schema_name(), "WaQrCodeState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaQrCodeState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), std::any::TypeId::of::<WaQrCodeState>());
    }
}
