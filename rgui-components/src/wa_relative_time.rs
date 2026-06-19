/// Translated from Web Awesome wa-relative-time
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

/// Web Awesome wa-relative-time 组件状态。
///
/// 将日期显示为相对于当前时间的本地化短语，如 "3 hours ago" 或 "in 2 days"。
/// 纯计算组件，无交互事件。当前硬编码英文格式（对齐 WA `Intl.RelativeTimeFormat` 英文输出）。
/// Phase 0: sync 属性忽略（不自动更新）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaRelativeTimeState {
    /// ISO 8601 日期字符串（如 "2024-01-15T10:30:00"）
    pub date: String,
    /// 格式风格：long | short | narrow
    pub format: String,
    /// 自动模式：always（始终显示 "X days ago"）| auto（可能显示 "yesterday"/"tomorrow"）
    pub numeric: String,
    /// 是否随时间自动更新（Phase 0 忽略）
    pub sync: bool,
}

impl WaRelativeTimeState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            date: String::new(),
            format: "long".into(),
            numeric: "auto".into(),
            sync: false,
        }
    }
}

// ============================================================================
// Relative Time Logic
// ============================================================================

/// 可用的时间单位及其阈值（对齐 WA `availableUnits`）。
struct UnitConfig {
    /// 该单位的最大毫秒差
    max: f64,
    /// 该单位的毫秒值
    value: f64,
    /// 单位名称
    name: &'static str,
    /// short 缩写
    short: &'static str,
    /// narrow 缩写
    narrow: &'static str,
}

const AVAILABLE_UNITS: &[UnitConfig] = &[
    UnitConfig {
        max: 2_760_000.0,
        value: 60_000.0,
        name: "minute",
        short: "min.",
        narrow: "m",
    },
    UnitConfig {
        max: 72_000_000.0,
        value: 3_600_000.0,
        name: "hour",
        short: "hr.",
        narrow: "h",
    },
    UnitConfig {
        max: 518_400_000.0,
        value: 86_400_000.0,
        name: "day",
        short: "day",
        narrow: "d",
    },
    UnitConfig {
        max: 2_419_200_000.0,
        value: 604_800_000.0,
        name: "week",
        short: "wk.",
        narrow: "w",
    },
    UnitConfig {
        max: 28_512_000_000.0,
        value: 2_592_000_000.0,
        name: "month",
        short: "mo.",
        narrow: "m",
    },
    UnitConfig {
        max: f64::INFINITY,
        value: 31_536_000_000.0,
        name: "year",
        short: "yr.",
        narrow: "y",
    },
];

/// 简单 ISO 8601 解析（仅支持 YYYY-MM-DDTHH:MM:SS 和 YYYY-MM-DD 格式）。
/// 返回自 Unix epoch 以来的毫秒数，解析失败返回 f64::NAN。
fn parse_iso_8601(s: &str) -> f64 {
    if s.is_empty() {
        return f64::NAN;
    }

    let s = s.trim();

    // 尝试解析 YYYY-MM-DDTHH:MM:SS 或 YYYY-MM-DDTHH:MM:SS.sssZ
    let (date_part, time_part) = if let Some(idx) = s.find('T') {
        (&s[..idx], Some(&s[idx + 1..]))
    } else if s.len() >= 10 && s.chars().filter(|&c| c == '-').count() == 2 {
        (s, None)
    } else {
        return f64::NAN;
    };

    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return f64::NAN;
    }

    let year: i32 = date_parts[0].parse().unwrap_or(0);
    let month: u32 = date_parts[1].parse().unwrap_or(1);
    let day: u32 = date_parts[2].parse().unwrap_or(1);

    if year == 0 || month == 0 || month > 12 || day == 0 || day > 31 {
        return f64::NAN;
    }

    // 简化：使用天数近似计算（不精确但足够用于相对时间显示）
    let days_before_month: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut total_days = (year as i64 - 1970) * 365;
    // 闰年修正
    total_days +=
        ((year as i64 - 1969) / 4) - ((year as i64 - 1901) / 100) + ((year as i64 - 1601) / 400);
    total_days += days_before_month[(month - 1) as usize] as i64;
    total_days += (day - 1) as i64;
    // 当年是闰年且月份 > 2 时加一天
    if month > 2 && (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)) {
        total_days += 1;
    }

    let mut ms = total_days as f64 * 86_400_000.0;

    // 时间部分
    if let Some(t) = time_part {
        // 去除时区后缀（Z/+00:00/-00:00）
        let t_clean = if let Some(z) = t.find('Z') {
            &t[..z]
        } else if let Some(p) = t.find('+') {
            &t[..p]
        } else if t.len() > 8 {
            // 可能有 - 时区偏移
            if let Some(m) = t[2..].find('-') {
                &t[..m + 2]
            } else {
                t
            }
        } else {
            t
        };

        let time_parts: Vec<&str> = t_clean.split(':').collect();
        if time_parts.len() >= 2 {
            let hours: f64 = time_parts[0].parse().unwrap_or(0.0);
            let minutes: f64 = time_parts[1].parse().unwrap_or(0.0);
            let seconds: f64 = time_parts
                .get(2)
                .map(|s| s.split('.').next().unwrap_or("0"))
                .unwrap_or("0")
                .parse()
                .unwrap_or(0.0);
            ms += hours * 3_600_000.0 + minutes * 60_000.0 + seconds * 1_000.0;
        }
    }

    ms
}

/// 获取当前时间的 Unix 毫秒数。
fn now_ms() -> f64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// 将相对时间格式化为英文字符串（硬编码，对齐 WA `Intl.RelativeTimeFormat`）。
fn relative_time(date_str: &str, format: &str, numeric: &str) -> String {
    let then_ms = parse_iso_8601(date_str);
    if then_ms.is_nan() {
        return String::new();
    }

    let now = now_ms();
    let diff = then_ms - now; // 正数 = 未来，负数 = 过去
    let abs_diff = diff.abs();

    // 找到合适的单位
    let unit = AVAILABLE_UNITS
        .iter()
        .find(|u| abs_diff < u.max)
        .unwrap_or(&AVAILABLE_UNITS[AVAILABLE_UNITS.len() - 1]);

    let value = (diff / unit.value).round() as i64;
    let abs_value = value.unsigned_abs();

    // 特殊处理 "auto" 模式下的 day 单位
    if numeric == "auto" && unit.name == "day" && abs_value <= 1 {
        if value == 0 {
            return "today".to_string();
        } else if value == -1 {
            return "yesterday".to_string();
        } else if value == 1 {
            return "tomorrow".to_string();
        }
    }

    let unit_str = match format {
        "short" => unit.short,
        "narrow" => unit.narrow,
        _ => unit.name,
    };

    if value == 0 {
        return match format {
            "short" => format!("0 {} ago", unit_str),
            "narrow" => format!("0{} ago", unit_str),
            _ => format!("0 {} ago", unit.name),
        };
    }

    match format {
        "short" => {
            if value < 0 {
                format!("{} {} ago", abs_value, unit_str)
            } else {
                format!("in {} {}", abs_value, unit_str)
            }
        },
        "narrow" => {
            if value < 0 {
                format!("{}{} ago", abs_value, unit_str)
            } else {
                format!("in {}{}", abs_value, unit_str)
            }
        },
        _ => {
            // long
            let unit_name = if abs_value == 1 {
                unit.name.to_string()
            } else {
                format!("{}s", unit.name)
            };
            if value < 0 {
                format!("{} {} ago", abs_value, unit_name)
            } else {
                format!("in {} {}", abs_value, unit_name)
            }
        },
    }
}

// ============================================================================
// Message
// ============================================================================

/// RelativeTime 无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaRelativeTimeMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaRelativeTime;

impl WidgetSpec for WaRelativeTime {
    type State = WaRelativeTimeState;
    type Message = WaRelativeTimeMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaRelativeTime"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaRelativeTime")
            .prop("date", PropValue::str(state.date.as_str()))
            .prop("format", PropValue::str(state.format.as_str()))
            .prop("numeric", PropValue::str(state.numeric.as_str()))
            .prop("sync", PropValue::Bool(state.sync))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaRelativeTimeMessage::NoOp => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let text = relative_time(&state.date, &state.format, &state.numeric);
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
        let text = relative_time(&state.date, &state.format, &state.numeric);
        if text.is_empty() {
            return;
        }
        let font_size: f32 = 16.0;
        let color = Color::new(0.1, 0.1, 0.1, 1.0);
        ctx.draw_text(&text, bounds, color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let text = relative_time(&state.date, &state.format, &state.numeric);
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

    // ── parse_iso_8601 测试 ──

    #[test]
    fn parse_empty_returns_nan() {
        assert!(parse_iso_8601("").is_nan());
    }

    #[test]
    fn parse_valid_date() {
        let ms = parse_iso_8601("2024-01-15T10:30:00");
        assert!(!ms.is_nan(), "应成功解析 ISO 日期");
        assert!(ms > 0.0, "时间戳应 > 0");
    }

    #[test]
    fn parse_date_only() {
        let ms = parse_iso_8601("2024-01-15");
        assert!(!ms.is_nan());
    }

    #[test]
    fn parse_invalid_returns_nan() {
        assert!(parse_iso_8601("not-a-date").is_nan());
        assert!(parse_iso_8601("2024-13-01").is_nan());
    }

    // ── relative_time 单元测试 ──

    #[test]
    fn relative_time_empty_date_returns_empty() {
        assert_eq!(relative_time("", "long", "always"), "");
    }

    #[test]
    fn relative_time_invalid_date_returns_empty() {
        assert_eq!(relative_time("bad", "long", "always"), "");
    }

    #[test]
    fn relative_time_past_date_returns_ago() {
        // 2020-01-01 显然在过去
        let result = relative_time("2020-01-01T00:00:00", "long", "always");
        assert!(!result.is_empty());
        assert!(result.contains("ago"), "应为过去时态，实际: {result}");
    }

    #[test]
    fn relative_time_future_date_returns_in() {
        // 2099-01-01 显然在未来
        let result = relative_time("2099-01-01T00:00:00", "long", "always");
        assert!(!result.is_empty());
        assert!(result.contains("in "), "应为将来时态，实际: {result}");
    }

    #[test]
    fn relative_time_long_format() {
        let result = relative_time("2020-01-01T00:00:00", "long", "always");
        // 应包含 "years" 或 "year"
        assert!(result.contains("year"), "应为 years/year，实际: {result}");
    }

    #[test]
    fn relative_time_short_format() {
        let result = relative_time("2020-01-01T00:00:00", "short", "always");
        assert!(!result.is_empty());
    }

    #[test]
    fn relative_time_narrow_format() {
        let result = relative_time("2020-01-01T00:00:00", "narrow", "always");
        assert!(!result.is_empty());
    }

    #[test]
    fn relative_time_auto_numeric() {
        // 测试 auto 模式也能正常返回
        let result = relative_time("2020-01-01T00:00:00", "long", "auto");
        assert!(!result.is_empty());
    }

    // ── WidgetSpec 测试 ──

    #[test]
    fn name() {
        assert_eq!(WaRelativeTime.name(), "rgui_components::WaRelativeTime");
    }

    #[test]
    fn default_state() {
        let state = WaRelativeTimeState::new();
        assert_eq!(state.date, "");
        assert_eq!(state.format, "long");
        assert_eq!(state.numeric, "auto");
        assert!(!state.sync);
    }

    #[test]
    fn view_has_props() {
        let state = WaRelativeTimeState::new();
        let v = WaRelativeTime.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaRelativeTime");
        assert!(v.props.contains_key("date"));
        assert!(v.props.contains_key("format"));
    }

    #[test]
    fn view_date_prop() {
        let mut state = WaRelativeTimeState::new();
        state.date = "2024-01-15".into();
        let v = WaRelativeTime.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("date").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "2024-01-15"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaRelativeTimeState::new();
        WaRelativeTime.update(
            WaRelativeTimeMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_non_zero_for_valid_date() {
        let mut state = WaRelativeTimeState::new();
        state.date = "2020-01-01T00:00:00".into();
        let size = WaRelativeTime.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width > 0.0, "宽度应 > 0，实际 {size:?}");
        assert!(size.height > 0.0, "高度应 > 0，实际 {size:?}");
    }

    #[test]
    fn measure_empty_date_returns_zero() {
        let state = WaRelativeTimeState::new();
        let size = WaRelativeTime.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_produces_ops() {
        let mut state = WaRelativeTimeState::new();
        state.date = "2020-01-01T00:00:00".into();
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaRelativeTime.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应产生至少 1 个绘制操作");
    }

    #[test]
    fn paint_empty_date_no_ops() {
        let state = WaRelativeTimeState::new();
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaRelativeTime.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "空日期不应产生绘制操作");
    }

    #[test]
    fn accessibility_has_label() {
        let mut state = WaRelativeTimeState::new();
        state.date = "2020-01-01T00:00:00".into();
        let node = WaRelativeTime.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_some(), "应为格式化文本提供标签");
    }

    #[test]
    fn accessibility_empty_no_label() {
        let state = WaRelativeTimeState::new();
        let node = WaRelativeTime.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_none());
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaRelativeTimeMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaRelativeTimeState::schema_name(), "WaRelativeTimeState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaRelativeTimeState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaRelativeTimeState>());
    }
}
