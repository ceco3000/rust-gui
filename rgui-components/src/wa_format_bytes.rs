/// Translated from Web Awesome wa-format-bytes
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

/// Web Awesome wa-format-bytes 组件状态。
///
/// 将字节数格式化为人类可读的字符串，支持 byte/bit 单位和 long/short/narrow 显示风格。
/// 纯计算组件，无交互事件。当前硬编码英文格式（对齐 WA `localize.number` 英文输出）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaFormatBytesState {
    /// 要格式化的字节数
    pub value: f64,
    /// 单位类型：byte | bit
    pub unit: String,
    /// 显示风格：long | short | narrow
    pub display: String,
}

impl WaFormatBytesState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: 0.0,
            unit: "byte".into(),
            display: "short".into(),
        }
    }
}

// ============================================================================
// Format Bytes Logic
// ============================================================================

/// 格式化字节数为人类可读字符串（英文硬编码，对齐 WA `Intl.NumberFormat` `style: 'unit'`）。
///
/// 基于 WA 原逻辑：
/// 1. 根据单位类型（byte/bit）确定前缀数组（bit 不含 peta）
/// 2. 计算 SI 指数：`floor(log10(value) / 3)`，限定在前缀数组范围内
/// 3. 缩放值：`value / 1000^index`，保留 3 位有效数字
/// 4. 按 display 模式组合数字与单位
fn format_bytes(value: f64, unit: &str, display: &str) -> String {
    if value.is_nan() {
        return String::new();
    }

    let prefixes: &[&str] = if unit == "bit" {
        &["", "kilo", "mega", "giga", "tera"]
    } else {
        &["", "kilo", "mega", "giga", "tera", "peta"]
    };

    let index = if value == 0.0 {
        0
    } else {
        let i = (value.abs().log10() / 3.0).floor() as i32;
        i.max(0).min(prefixes.len() as i32 - 1) as usize
    };

    let scaled = value / 1000_f64.powi(index as i32);
    // 3 位有效数字后去除尾部零和小数点（对齐 WA parseFloat(toPrecision(3))）
    let formatted = format!("{:.3}", scaled);
    let formatted = formatted.trim_end_matches('0').trim_end_matches('.');

    match display {
        "long" => {
            let unit_name = match (unit, index, formatted) {
                ("byte", 0, "1") => "byte",
                ("byte", 0, _) => "bytes",
                ("byte", 1, _) => "kilobytes",
                ("byte", 2, _) => "megabytes",
                ("byte", 3, _) => "gigabytes",
                ("byte", 4, _) => "terabytes",
                ("byte", 5, _) => "petabytes",
                ("bit", 0, "1") => "bit",
                ("bit", 0, _) => "bits",
                ("bit", 1, _) => "kilobits",
                ("bit", 2, _) => "megabits",
                ("bit", 3, _) => "gigabits",
                ("bit", 4, _) => "terabits",
                _ => "bytes",
            };
            format!("{} {}", formatted, unit_name)
        },
        "narrow" => {
            let (prefix, unit_suffix) = match (unit, index) {
                ("byte", 0) => ("", "B"),
                ("byte", 1) => ("k", "B"),
                ("byte", 2) => ("M", "B"),
                ("byte", 3) => ("G", "B"),
                ("byte", 4) => ("T", "B"),
                ("byte", 5) => ("P", "B"),
                ("bit", 0) => ("", "bit"),
                ("bit", 1) => ("k", "b"),
                ("bit", 2) => ("M", "b"),
                ("bit", 3) => ("G", "b"),
                ("bit", 4) => ("T", "b"),
                _ => ("", "B"),
            };
            format!("{}{}{}", formatted, prefix, unit_suffix)
        },
        _ => {
            // "short" (default) — 紧凑显示，数字和单位间有空格
            let (prefix, unit_suffix) = match (unit, index) {
                ("byte", 0) => ("", "byte"),
                ("byte", 1) => ("k", "B"),
                ("byte", 2) => ("M", "B"),
                ("byte", 3) => ("G", "B"),
                ("byte", 4) => ("T", "B"),
                ("byte", 5) => ("P", "B"),
                ("bit", 0) => ("", "bit"),
                ("bit", 1) => ("k", "b"),
                ("bit", 2) => ("M", "b"),
                ("bit", 3) => ("G", "b"),
                ("bit", 4) => ("T", "b"),
                _ => ("", "B"),
            };
            format!("{} {}{}", formatted, prefix, unit_suffix)
        },
    }
}

// ============================================================================
// Message
// ============================================================================

/// FormatBytes 无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaFormatBytesMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaFormatBytes;

impl WidgetSpec for WaFormatBytes {
    type State = WaFormatBytesState;
    type Message = WaFormatBytesMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaFormatBytes"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaFormatBytes")
            .prop(
                "value",
                PropValue::Float(ordered_float::OrderedFloat(state.value)),
            )
            .prop("unit", PropValue::str(state.unit.as_str()))
            .prop("display", PropValue::str(state.display.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaFormatBytesMessage::NoOp => {},
        }
    }

    /// 根据格式化后的文本估算尺寸。
    ///
    /// 拉丁文字宽度按每字符 ≈ 0.55 × font_size 估算，
    /// 行高按 1.4 × font_size 估算。
    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let text = format_bytes(state.value, &state.unit, &state.display);
        if text.is_empty() {
            return Size::ZERO;
        }
        let font_size: f64 = 16.0;
        let char_width: f64 = font_size * 0.55;
        let text_width = text.len() as f64 * char_width;
        let line_height: f64 = font_size * 1.4;

        let w = text_width.clamp(c.min_width, c.max_width);
        let h = line_height.clamp(c.min_height, c.max_height);
        Size::new(w, h)
    }

    /// 绘制格式化后的文本。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let text = format_bytes(state.value, &state.unit, &state.display);
        if text.is_empty() {
            return;
        }
        let font_size: f32 = 16.0;
        let color = Color::new(0.1, 0.1, 0.1, 1.0); // 近黑色文字
        ctx.draw_text(&text, bounds, color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let text = format_bytes(state.value, &state.unit, &state.display);
        if text.is_empty() {
            AccessibilityNode::none()
        } else {
            AccessibilityNode::none().label(text)
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

    // ── format_bytes 单元测试 ──

    #[test]
    fn format_nan_returns_empty() {
        assert_eq!(format_bytes(f64::NAN, "byte", "short"), "");
        assert_eq!(format_bytes(f64::NAN, "bit", "long"), "");
    }

    #[test]
    fn format_zero_short() {
        assert_eq!(format_bytes(0.0, "byte", "short"), "0 byte");
    }

    #[test]
    fn format_zero_long() {
        assert_eq!(format_bytes(0.0, "byte", "long"), "0 bytes");
    }

    #[test]
    fn format_zero_narrow() {
        assert_eq!(format_bytes(0.0, "byte", "narrow"), "0B");
    }

    #[test]
    fn format_bytes_short() {
        assert_eq!(format_bytes(12.0, "byte", "short"), "12 byte");
        assert_eq!(format_bytes(1200.0, "byte", "short"), "1.2 kB");
        assert_eq!(format_bytes(1_200_000.0, "byte", "short"), "1.2 MB");
        assert_eq!(format_bytes(1_200_000_000.0, "byte", "short"), "1.2 GB");
        assert_eq!(format_bytes(1_200_000_000_000.0, "byte", "short"), "1.2 TB");
        assert_eq!(
            format_bytes(1_200_000_000_000_000.0, "byte", "short"),
            "1.2 PB"
        );
    }

    #[test]
    fn format_bytes_long() {
        assert_eq!(format_bytes(12.0, "byte", "long"), "12 bytes");
        assert_eq!(format_bytes(1200.0, "byte", "long"), "1.2 kilobytes");
        assert_eq!(format_bytes(1_200_000.0, "byte", "long"), "1.2 megabytes");
        assert_eq!(
            format_bytes(1_200_000_000.0, "byte", "long"),
            "1.2 gigabytes"
        );
        assert_eq!(
            format_bytes(1_200_000_000_000.0, "byte", "long"),
            "1.2 terabytes"
        );
        assert_eq!(
            format_bytes(1_200_000_000_000_000.0, "byte", "long"),
            "1.2 petabytes"
        );
    }

    #[test]
    fn format_bytes_narrow() {
        assert_eq!(format_bytes(12.0, "byte", "narrow"), "12B");
        assert_eq!(format_bytes(1200.0, "byte", "narrow"), "1.2kB");
        assert_eq!(format_bytes(1_200_000.0, "byte", "narrow"), "1.2MB");
        assert_eq!(format_bytes(1_200_000_000.0, "byte", "narrow"), "1.2GB");
        assert_eq!(format_bytes(1_200_000_000_000.0, "byte", "narrow"), "1.2TB");
        assert_eq!(
            format_bytes(1_200_000_000_000_000.0, "byte", "narrow"),
            "1.2PB"
        );
    }

    #[test]
    fn format_bits_short() {
        assert_eq!(format_bytes(12.0, "bit", "short"), "12 bit");
        assert_eq!(format_bytes(1200.0, "bit", "short"), "1.2 kb");
        assert_eq!(format_bytes(1_200_000.0, "bit", "short"), "1.2 Mb");
        assert_eq!(format_bytes(1_200_000_000.0, "bit", "short"), "1.2 Gb");
        assert_eq!(format_bytes(1_200_000_000_000.0, "bit", "short"), "1.2 Tb");
    }

    #[test]
    fn format_bits_long() {
        assert_eq!(format_bytes(12.0, "bit", "long"), "12 bits");
        assert_eq!(format_bytes(1200.0, "bit", "long"), "1.2 kilobits");
        assert_eq!(format_bytes(1_200_000.0, "bit", "long"), "1.2 megabits");
        assert_eq!(format_bytes(1_200_000_000.0, "bit", "long"), "1.2 gigabits");
        assert_eq!(
            format_bytes(1_200_000_000_000.0, "bit", "long"),
            "1.2 terabits"
        );
    }

    #[test]
    fn format_bits_narrow() {
        assert_eq!(format_bytes(12.0, "bit", "narrow"), "12bit");
        assert_eq!(format_bytes(1200.0, "bit", "narrow"), "1.2kb");
        assert_eq!(format_bytes(1_200_000.0, "bit", "narrow"), "1.2Mb");
        assert_eq!(format_bytes(1_200_000_000.0, "bit", "narrow"), "1.2Gb");
        assert_eq!(format_bytes(1_200_000_000_000.0, "bit", "narrow"), "1.2Tb");
    }

    #[test]
    fn format_edge_value_one_byte() {
        // 值 = 1 时 long 显示单数
        assert_eq!(format_bytes(1.0, "byte", "long"), "1 byte");
        assert_eq!(format_bytes(1.0, "byte", "short"), "1 byte");
        assert_eq!(format_bytes(1.0, "byte", "narrow"), "1B");
    }

    #[test]
    fn format_edge_value_one_bit() {
        assert_eq!(format_bytes(1.0, "bit", "long"), "1 bit");
        assert_eq!(format_bytes(1.0, "bit", "short"), "1 bit");
        assert_eq!(format_bytes(1.0, "bit", "narrow"), "1bit");
    }

    #[test]
    fn format_exceeds_max_prefix() {
        // 值超出 peta 范围，应使用最大前缀
        let huge = 1_200_000_000_000_000_000.0; // 1.2 exa
        assert!(format_bytes(huge, "byte", "short").contains("PB"));
        assert!(format_bytes(huge, "bit", "short").contains("Tb"));
    }

    // ── WidgetSpec 测试 ──

    #[test]
    fn name() {
        assert_eq!(WaFormatBytes.name(), "rgui_components::WaFormatBytes");
    }

    #[test]
    fn default_state() {
        let state = WaFormatBytesState::new();
        assert_eq!(state.value, 0.0);
        assert_eq!(state.unit, "byte");
        assert_eq!(state.display, "short");
    }

    #[test]
    fn view_has_props() {
        let state = WaFormatBytesState::new();
        let v = WaFormatBytes.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaFormatBytes");
        assert!(v.props.contains_key("value"));
        assert!(v.props.contains_key("unit"));
        assert!(v.props.contains_key("display"));
    }

    #[test]
    fn view_value_prop() {
        let mut state = WaFormatBytesState::new();
        state.value = 1200.0;
        let v = WaFormatBytes.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("value").unwrap() {
            PropValue::Float(f) => assert!((f.0 - 1200.0).abs() < 0.01),
            _ => panic!("expected Float prop"),
        }
    }

    #[test]
    fn view_unit_prop() {
        let mut state = WaFormatBytesState::new();
        state.unit = "bit".into();
        let v = WaFormatBytes.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("unit").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "bit"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaFormatBytesState::new();
        WaFormatBytes.update(
            WaFormatBytesMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_non_zero_for_valid_value() {
        let state = WaFormatBytesState::new(); // value=0, short → "0 byte"
        let size = WaFormatBytes.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width > 0.0, "宽度应 > 0，实际 {size:?}");
        assert!(size.height > 0.0, "高度应 > 0，实际 {size:?}");
    }

    #[test]
    fn measure_nan_returns_zero() {
        let mut state = WaFormatBytesState::new();
        state.value = f64::NAN;
        let size = WaFormatBytes.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn measure_clamped_by_constraints() {
        let state = WaFormatBytesState::new();
        let size = WaFormatBytes.measure(
            &state,
            BoxConstraints::new(0.0, 10.0, 0.0, 10.0),
            &MeasureContext::default(),
        );
        assert!(size.width <= 10.0, "宽度应被约束限制，实际 {size:?}");
        assert!(size.height <= 10.0, "高度应被约束限制，实际 {size:?}");
    }

    #[test]
    fn paint_produces_ops() {
        let state = WaFormatBytesState::new();
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaFormatBytes.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应产生至少 1 个绘制操作");
    }

    #[test]
    fn paint_nan_produces_no_ops() {
        let mut state = WaFormatBytesState::new();
        state.value = f64::NAN;
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaFormatBytes.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "NaN 不应产生绘制操作");
    }

    #[test]
    fn accessibility_has_label() {
        let state = WaFormatBytesState::new();
        let node = WaFormatBytes.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_some(), "应为格式化文本提供标签");
    }

    #[test]
    fn accessibility_nan_no_label() {
        let mut state = WaFormatBytesState::new();
        state.value = f64::NAN;
        let node = WaFormatBytes.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_none());
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaFormatBytesMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaFormatBytesState::schema_name(), "WaFormatBytesState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaFormatBytesState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaFormatBytesState>());
    }
}
