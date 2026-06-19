/// Translated from Web Awesome wa-format-number
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-format-number 组件状态。
///
/// 将数字格式化为人类可读字符串，支持 decimal/currency/percent 三种风格。
/// 纯计算组件，无交互事件。当前硬编码英文格式（对齐 WA `Intl.NumberFormat` 英文输出）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaFormatNumberState {
    /// 要格式化的数字
    pub value: f64,
    /// 格式风格：decimal | currency | percent
    pub style: String,
    /// 是否禁用分组分隔符
    pub without_grouping: bool,
    /// 货币代码（ISO 4217），如 USD
    pub currency: String,
    /// 货币显示方式：symbol | narrowSymbol | code | name
    pub currency_display: String,
    /// 最小整数位数（1–21）
    pub minimum_integer_digits: Option<usize>,
    /// 最小小数位数（0–100）
    pub minimum_fraction_digits: Option<usize>,
    /// 最大小数位数（0–100）
    pub maximum_fraction_digits: Option<usize>,
    /// 最小有效位数（1–21）
    pub minimum_significant_digits: Option<usize>,
    /// 最大有效位数（1–21）
    pub maximum_significant_digits: Option<usize>,
}

impl WaFormatNumberState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: 0.0,
            style: "decimal".into(),
            without_grouping: false,
            currency: "USD".into(),
            currency_display: "symbol".into(),
            minimum_integer_digits: None,
            minimum_fraction_digits: None,
            maximum_fraction_digits: None,
            minimum_significant_digits: None,
            maximum_significant_digits: None,
        }
    }
}

// ============================================================================
// Format Number Logic
// ============================================================================

/// 获取货币符号（硬编码常见货币，英文）。
fn currency_symbol(code: &str, display: &str) -> String {
    match (code, display) {
        ("USD", "symbol") | ("USD", "narrowSymbol") => "$".into(),
        ("EUR", "symbol") | ("EUR", "narrowSymbol") => "€".into(),
        ("GBP", "symbol") | ("GBP", "narrowSymbol") => "£".into(),
        ("JPY", "symbol") | ("JPY", "narrowSymbol") => "¥".into(),
        ("CNY", "symbol") | ("CNY", "narrowSymbol") => "¥".into(),
        ("KRW", "symbol") | ("KRW", "narrowSymbol") => "₩".into(),
        ("INR", "symbol") | ("INR", "narrowSymbol") => "₹".into(),
        (_, "code") => code.to_string(),
        (_, "name") => code.to_string(),
        _ => "$".into(),
    }
}

/// 带分组分隔符和小数位数的格式化。
fn format_decimal(value: f64, min_frac: usize, max_frac: usize, use_grouping: bool) -> String {
    // 处理负数
    let sign = if value < 0.0 { "-" } else { "" };
    let abs_value = value.abs();

    // 按要求小数位数格式化
    let formatted = format!("{:.prec$}", abs_value, prec = max_frac);
    let parts: Vec<&str> = formatted.split('.').collect();
    let int_part = parts[0];
    let mut frac_part = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

    // 截断到 max_frac
    if frac_part.len() > max_frac {
        frac_part.truncate(max_frac);
    }
    // 去除尾部零直到满足 min_frac
    while frac_part.len() > min_frac && frac_part.ends_with('0') {
        frac_part.pop();
    }

    // 分组分隔符
    let int_str = if use_grouping {
        let mut result = String::new();
        let len = int_part.len();
        for (i, ch) in int_part.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                result.push(',');
            }
            result.push(ch);
        }
        result
    } else {
        int_part.to_string()
    };

    if frac_part.is_empty() {
        format!("{}{}", sign, int_str)
    } else {
        format!("{}{}.{}", sign, int_str, frac_part)
    }
}

/// 使用有效数字格式化（简化实现）。
fn format_significant(value: f64, min_sig: usize, max_sig: usize, use_grouping: bool) -> String {
    if value == 0.0 {
        let zeros = "0".repeat(min_sig);
        return if min_sig > 1 {
            format!("{}.{}", zeros, &zeros[1..])
        } else {
            "0".to_string()
        };
    }

    let sign = if value < 0.0 { "-" } else { "" };
    let abs_value = value.abs();

    // 计算数量级
    let magnitude = if abs_value >= 1.0 {
        (abs_value.log10().floor() as i32) + 1
    } else {
        abs_value.log10().floor() as i32
    };

    // 缩放到 max_sig 位有效数字
    let scale = 10_f64.powi(max_sig as i32 - magnitude);
    let rounded = (abs_value * scale).round() / scale;

    // 格式化为字符串
    let formatted = format!(
        "{:.prec$}",
        rounded,
        prec = (max_sig as i32 - magnitude).max(0) as usize
    );
    let parts: Vec<&str> = formatted.split('.').collect();
    let int_part = parts[0];
    let mut frac_part = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

    // 确保至少 min_sig 位有效数字
    let sig_digits = int_part.len() + frac_part.len();
    while sig_digits < min_sig && !(int_part == "0" && frac_part.is_empty()) {
        frac_part.push('0');
    }

    let int_str = if use_grouping {
        let mut result = String::new();
        let len = int_part.len();
        for (i, ch) in int_part.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                result.push(',');
            }
            result.push(ch);
        }
        result
    } else {
        int_part.to_string()
    };

    if frac_part.is_empty() {
        format!("{}{}", sign, int_str)
    } else {
        format!("{}{}.{}", sign, int_str, frac_part)
    }
}

/// 格式化数字为人类可读字符串（英文硬编码，对齐 WA `Intl.NumberFormat`）。
fn format_number(state: &WaFormatNumberState) -> String {
    if state.value.is_nan() {
        return String::new();
    }

    let use_grouping = !state.without_grouping;
    let min_int = state.minimum_integer_digits.unwrap_or(1);

    match state.style.as_str() {
        "percent" => {
            let pct = state.value * 100.0;
            let min_frac = state.minimum_fraction_digits.unwrap_or(0);
            let max_frac = state.maximum_fraction_digits.unwrap_or(0);
            let mut s = format_decimal(pct, min_frac, max_frac, use_grouping);
            // 保证最小整数位数
            if let Some(dot_pos) = s.find('.') {
                let int_len = if s.starts_with('-') {
                    dot_pos - 1
                } else {
                    dot_pos
                };
                if int_len < min_int {
                    s = format!(
                        "{}{}",
                        "0".repeat(min_int - int_len),
                        s.trim_start_matches('-')
                    );
                    if s.starts_with('-') {
                        // 负数处理
                    }
                }
            }
            format!("{}%", s)
        },
        "currency" => {
            let symbol = currency_symbol(&state.currency, &state.currency_display);
            let min_frac = state.minimum_fraction_digits.unwrap_or(2);
            let max_frac = state.maximum_fraction_digits.unwrap_or(2);
            let s = format_decimal(state.value, min_frac, max_frac, use_grouping);
            format!("{}{}", symbol, s)
        },
        _ => {
            // "decimal" — 默认
            let min_frac = state.minimum_fraction_digits.unwrap_or(0);
            let max_frac = state.maximum_fraction_digits.unwrap_or(3);
            if state.minimum_significant_digits.is_some()
                || state.maximum_significant_digits.is_some()
            {
                let min_sig = state.minimum_significant_digits.unwrap_or(1);
                let max_sig = state.maximum_significant_digits.unwrap_or(6);
                format_significant(state.value, min_sig, max_sig, use_grouping)
            } else {
                format_decimal(state.value, min_frac, max_frac, use_grouping)
            }
        },
    }
}

// ============================================================================
// Message
// ============================================================================

/// FormatNumber 无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaFormatNumberMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaFormatNumber;

impl WidgetSpec for WaFormatNumber {
    type State = WaFormatNumberState;
    type Message = WaFormatNumberMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaFormatNumber"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        use ordered_float::OrderedFloat;
        WidgetView::new("rgui_components::WaFormatNumber")
            .prop("value", PropValue::Float(OrderedFloat(state.value)))
            .prop("style", PropValue::str(state.style.as_str()))
            .prop("without_grouping", PropValue::Bool(state.without_grouping))
            .prop("currency", PropValue::str(state.currency.as_str()))
            .prop(
                "currency_display",
                PropValue::str(state.currency_display.as_str()),
            )
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaFormatNumberMessage::NoOp => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let text = format_number(state);
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

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let text = format_number(state);
        if text.is_empty() {
            return;
        }
        let font_size: f32 = 16.0;
        let color = Color::new(0.1, 0.1, 0.1, 1.0);
        ctx.draw_text(&text, bounds, color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let text = format_number(state);
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

    // ── format_number 单元测试 ──

    #[test]
    fn format_nan_returns_empty() {
        let mut state = WaFormatNumberState::new();
        state.value = f64::NAN;
        assert_eq!(format_number(&state), "");
    }

    #[test]
    fn format_decimal_zero() {
        let state = WaFormatNumberState::new(); // value=0, decimal
        assert_eq!(format_number(&state), "0");
    }

    #[test]
    fn format_decimal_integer() {
        let mut state = WaFormatNumberState::new();
        state.value = 1234.0;
        assert_eq!(format_number(&state), "1,234");
    }

    #[test]
    fn format_decimal_with_fraction() {
        let mut state = WaFormatNumberState::new();
        state.value = 1234.567;
        assert_eq!(format_number(&state), "1,234.567");
    }

    #[test]
    fn format_decimal_negative() {
        let mut state = WaFormatNumberState::new();
        state.value = -1234.0;
        assert_eq!(format_number(&state), "-1,234");
    }

    #[test]
    fn format_decimal_without_grouping() {
        let mut state = WaFormatNumberState::new();
        state.value = 1234567.0;
        state.without_grouping = true;
        assert_eq!(format_number(&state), "1234567");
    }

    #[test]
    fn format_decimal_fraction_digits() {
        let mut state = WaFormatNumberState::new();
        state.value = 3.14159;
        state.minimum_fraction_digits = Some(2);
        state.maximum_fraction_digits = Some(4);
        assert_eq!(format_number(&state), "3.1416");
    }

    #[test]
    fn format_percent() {
        let mut state = WaFormatNumberState::new();
        state.style = "percent".into();
        state.value = 0.1234;
        // WA Intl.NumberFormat defaults to 0 fraction digits for percent
        assert_eq!(format_number(&state), "12%");
    }

    #[test]
    fn format_percent_100() {
        let mut state = WaFormatNumberState::new();
        state.style = "percent".into();
        state.value = 1.0;
        assert_eq!(format_number(&state), "100%");
    }

    #[test]
    fn format_currency_usd() {
        let mut state = WaFormatNumberState::new();
        state.style = "currency".into();
        state.value = 1234.56;
        assert_eq!(format_number(&state), "$1,234.56");
    }

    #[test]
    fn format_currency_eur() {
        let mut state = WaFormatNumberState::new();
        state.style = "currency".into();
        state.currency = "EUR".into();
        state.value = 99.9;
        assert_eq!(format_number(&state), "€99.90");
    }

    #[test]
    fn format_currency_code_display() {
        let mut state = WaFormatNumberState::new();
        state.style = "currency".into();
        state.currency = "USD".into();
        state.currency_display = "code".into();
        state.value = 42.0;
        assert_eq!(format_number(&state), "USD42.00");
    }

    // ── WidgetSpec 测试 ──

    #[test]
    fn name() {
        assert_eq!(WaFormatNumber.name(), "rgui_components::WaFormatNumber");
    }

    #[test]
    fn default_state() {
        let state = WaFormatNumberState::new();
        assert_eq!(state.value, 0.0);
        assert_eq!(state.style, "decimal");
        assert!(!state.without_grouping);
        assert_eq!(state.currency, "USD");
    }

    #[test]
    fn view_has_props() {
        let state = WaFormatNumberState::new();
        let v = WaFormatNumber.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaFormatNumber");
        assert!(v.props.contains_key("value"));
        assert!(v.props.contains_key("style"));
    }

    #[test]
    fn view_value_prop() {
        let mut state = WaFormatNumberState::new();
        state.value = 1234.0;
        let v = WaFormatNumber.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("value").unwrap() {
            PropValue::Float(f) => assert!((f.0 - 1234.0).abs() < 0.01),
            _ => panic!("expected Float prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaFormatNumberState::new();
        WaFormatNumber.update(
            WaFormatNumberMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_non_zero_for_valid_value() {
        let state = WaFormatNumberState::new();
        let size = WaFormatNumber.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width > 0.0, "宽度应 > 0，实际 {size:?}");
        assert!(size.height > 0.0, "高度应 > 0，实际 {size:?}");
    }

    #[test]
    fn measure_nan_returns_zero() {
        let mut state = WaFormatNumberState::new();
        state.value = f64::NAN;
        let size = WaFormatNumber.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_produces_ops() {
        let state = WaFormatNumberState::new();
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaFormatNumber.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应产生至少 1 个绘制操作");
    }

    #[test]
    fn paint_nan_produces_no_ops() {
        let mut state = WaFormatNumberState::new();
        state.value = f64::NAN;
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaFormatNumber.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "NaN 不应产生绘制操作");
    }

    #[test]
    fn accessibility_has_label() {
        let state = WaFormatNumberState::new();
        let node = WaFormatNumber.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_some(), "应为格式化文本提供标签");
    }

    #[test]
    fn accessibility_nan_no_label() {
        let mut state = WaFormatNumberState::new();
        state.value = f64::NAN;
        let node = WaFormatNumber.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_none());
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaFormatNumberMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaFormatNumberState::schema_name(), "WaFormatNumberState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaFormatNumberState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaFormatNumberState>());
    }
}
