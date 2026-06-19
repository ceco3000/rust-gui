/// Translated from Web Awesome wa-format-date
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

/// Web Awesome wa-format-date 组件状态。
///
/// 将日期/时间格式化为人类可读的字符串，支持多种格式选项。
/// 纯计算组件，无交互事件。当前硬编码英文格式（对齐 WA `Intl.DateTimeFormat` 英文输出）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaFormatDateState {
    /// 要格式化的日期（ISO 8601 字符串，如 "2026-06-20T12:00:00Z"）
    pub date: String,
    /// weekday 格式：narrow | short | long | ""
    pub weekday: String,
    /// era 格式：narrow | short | long | ""
    pub era: String,
    /// year 格式：numeric | 2-digit | ""
    pub year: String,
    /// month 格式：numeric | 2-digit | narrow | short | long | ""
    pub month: String,
    /// day 格式：numeric | 2-digit | ""
    pub day: String,
    /// hour 格式：numeric | 2-digit | ""
    pub hour: String,
    /// minute 格式：numeric | 2-digit | ""
    pub minute: String,
    /// second 格式：numeric | 2-digit | ""
    pub second: String,
    /// timeZoneName 格式：short | long | ""
    pub time_zone_name: String,
    /// 时区 ID（如 "America/New_York"），Phase 0 不处理
    pub time_zone: String,
    /// 小时制式：auto | 12 | 24
    pub hour_format: String,
}

impl WaFormatDateState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            date: String::new(),
            weekday: String::new(),
            era: String::new(),
            year: String::new(),
            month: String::new(),
            day: String::new(),
            hour: String::new(),
            minute: String::new(),
            second: String::new(),
            time_zone_name: String::new(),
            time_zone: String::new(),
            hour_format: "auto".into(),
        }
    }
}

// ============================================================================
// Date Parsing Helpers
// ============================================================================

/// 解析后的日期各部分。
#[derive(Debug, Clone, Copy)]
struct DateParts {
    year: i32,
    month: u32,  // 1-12
    day: u32,    // 1-31
    hour: u32,   // 0-23
    minute: u32, // 0-59
    second: u32, // 0-59
    valid: bool,
}

/// 解析 ISO 8601 日期字符串（简化版，支持 YYYY-MM-DD 和 YYYY-MM-DDTHH:MM:SS）。
fn parse_iso_date(s: &str) -> DateParts {
    if s.is_empty() || s.len() < 10 {
        return DateParts {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            valid: false,
        };
    }

    let year: i32 = match s[0..4].parse() {
        Ok(y) => y,
        Err(_) => {
            return DateParts {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                valid: false,
            };
        },
    };
    let month: u32 = match s[5..7].parse() {
        Ok(m) if m >= 1 && m <= 12 => m,
        _ => {
            return DateParts {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                valid: false,
            };
        },
    };
    let day: u32 = match s[8..10].parse() {
        Ok(d) if d >= 1 && d <= 31 => d,
        _ => {
            return DateParts {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                valid: false,
            };
        },
    };

    let (hour, minute, second) = if s.len() >= 19 && s.as_bytes().get(10) == Some(&b'T') {
        let h: u32 = match s[11..13].parse() {
            Ok(h) if h <= 23 => h,
            _ => {
                return DateParts {
                    year: 0,
                    month: 0,
                    day: 0,
                    hour: 0,
                    minute: 0,
                    second: 0,
                    valid: false,
                };
            },
        };
        let m: u32 = match s[14..16].parse() {
            Ok(m) if m <= 59 => m,
            _ => {
                return DateParts {
                    year: 0,
                    month: 0,
                    day: 0,
                    hour: 0,
                    minute: 0,
                    second: 0,
                    valid: false,
                };
            },
        };
        let sec: u32 = match s[17..19].parse() {
            Ok(s) if s <= 59 => s,
            _ => {
                return DateParts {
                    year: 0,
                    month: 0,
                    day: 0,
                    hour: 0,
                    minute: 0,
                    second: 0,
                    valid: false,
                };
            },
        };
        (h, m, sec)
    } else {
        (0, 0, 0)
    };

    DateParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
        valid: true,
    }
}

// ============================================================================
// Format Date Logic
// ============================================================================

/// 格式化日期为人类可读字符串（英文硬编码，对齐 WA `Intl.DateTimeFormat`）。
fn format_date(state: &WaFormatDateState) -> String {
    let parts = parse_iso_date(&state.date);
    if !parts.valid {
        return String::new();
    }

    let mut result = String::new();

    // Weekday
    if !state.weekday.is_empty() {
        let weekday_name = get_weekday_name(parts.year, parts.month, parts.day, &state.weekday);
        if !weekday_name.is_empty() {
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(&weekday_name);
            result.push(',');
        }
    }

    // Era — skip for simplicity (Phase 0), only handle AD dates

    // Month + Day + Year
    if !state.month.is_empty() || !state.day.is_empty() || !state.year.is_empty() {
        if !result.is_empty() {
            result.push(' ');
        }

        // Month
        if !state.month.is_empty() {
            let month_name = match state.month.as_str() {
                "long" => LONG_MONTHS[parts.month as usize - 1],
                "short" => SHORT_MONTHS[parts.month as usize - 1],
                "narrow" => NARROW_MONTHS[parts.month as usize - 1],
                "numeric" => "",
                "2-digit" => "",
                _ => SHORT_MONTHS[parts.month as usize - 1],
            };
            if month_name.is_empty() && (state.month == "numeric" || state.month == "2-digit") {
                if state.month == "2-digit" {
                    result.push_str(&format!("{:02}", parts.month));
                } else {
                    result.push_str(&format!("{}", parts.month));
                }
            } else {
                result.push_str(month_name);
            }
        }

        // Day
        if !state.day.is_empty() {
            if !state.month.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
            if state.day == "2-digit" {
                result.push_str(&format!("{:02}", parts.day));
            } else {
                result.push_str(&format!("{}", parts.day));
            }
        }

        // Year
        if !state.year.is_empty() {
            if !result.ends_with(' ') && (!state.month.is_empty() || !state.day.is_empty()) {
                result.push_str(", ");
            }
            if state.year == "2-digit" {
                result.push_str(&format!("{:02}", parts.year % 100));
            } else {
                result.push_str(&format!("{}", parts.year));
            }
        }
    }

    // Time
    let has_time = !state.hour.is_empty() || !state.minute.is_empty() || !state.second.is_empty();
    if has_time {
        if !result.is_empty() {
            result.push_str(", ");
        }

        let use_12h = state.hour_format == "12";
        let hour24 = parts.hour;

        let (display_hour, ampm) = if use_12h {
            if hour24 == 0 {
                (12, "AM")
            } else if hour24 < 12 {
                (hour24, "AM")
            } else if hour24 == 12 {
                (12, "PM")
            } else {
                (hour24 - 12, "PM")
            }
        } else {
            (hour24, "")
        };

        // Hour
        if !state.hour.is_empty() {
            if state.hour == "2-digit" {
                result.push_str(&format!("{:02}", display_hour));
            } else {
                result.push_str(&format!("{}", display_hour));
            }
        }

        // Minute
        if !state.minute.is_empty() {
            if !state.hour.is_empty() {
                result.push(':');
            }
            if state.minute == "2-digit" {
                result.push_str(&format!("{:02}", parts.minute));
            } else {
                result.push_str(&format!("{}", parts.minute));
            }
        }

        // Second
        if !state.second.is_empty() {
            if !state.hour.is_empty() || !state.minute.is_empty() {
                result.push(':');
            }
            if state.second == "2-digit" {
                result.push_str(&format!("{:02}", parts.second));
            } else {
                result.push_str(&format!("{}", parts.second));
            }
        }

        // AM/PM
        if use_12h && !ampm.is_empty() {
            result.push(' ');
            result.push_str(ampm);
        }

        // Timezone name
        if !state.time_zone_name.is_empty() {
            result.push_str(" GMT"); // Phase 0: hardcoded as default timezone indicator
        }
    }

    result.trim().to_string()
}

const LONG_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const NARROW_MONTHS: [&str; 12] = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];

const LONG_WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

const SHORT_WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

const NARROW_WEEKDAYS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];

/// 使用 Zeller 公式计算星期几（0=Sunday, 1=Monday, ..., 6=Saturday）
fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    let (m, y) = if month < 3 {
        (month + 12, year - 1)
    } else {
        (month, year)
    };
    let q = day as i32;
    let k = y % 100;
    let j = y / 100;
    let h = (q + (13 * (m as i32 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    // Zeller: 0=Saturday, 1=Sunday, ..., 6=Friday
    // Convert to 0=Sunday, 1=Monday, ..., 6=Saturday
    ((h + 6) % 7) as u32
}

fn get_weekday_name(year: i32, month: u32, day: u32, style: &str) -> String {
    let dow = day_of_week(year, month, day) as usize;
    match style {
        "long" => LONG_WEEKDAYS[dow].to_string(),
        "short" => SHORT_WEEKDAYS[dow].to_string(),
        "narrow" => NARROW_WEEKDAYS[dow].to_string(),
        _ => String::new(),
    }
}

// ============================================================================
// Message
// ============================================================================

/// FormatDate 无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaFormatDateMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaFormatDate;

impl WidgetSpec for WaFormatDate {
    type State = WaFormatDateState;
    type Message = WaFormatDateMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaFormatDate"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaFormatDate")
            .prop("date", PropValue::str(state.date.as_str()))
            .prop("weekday", PropValue::str(state.weekday.as_str()))
            .prop("era", PropValue::str(state.era.as_str()))
            .prop("year", PropValue::str(state.year.as_str()))
            .prop("month", PropValue::str(state.month.as_str()))
            .prop("day", PropValue::str(state.day.as_str()))
            .prop("hour", PropValue::str(state.hour.as_str()))
            .prop("minute", PropValue::str(state.minute.as_str()))
            .prop("second", PropValue::str(state.second.as_str()))
            .prop(
                "time-zone-name",
                PropValue::str(state.time_zone_name.as_str()),
            )
            .prop("time-zone", PropValue::str(state.time_zone.as_str()))
            .prop("hour-format", PropValue::str(state.hour_format.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaFormatDateMessage::NoOp => {},
        }
    }

    /// 根据格式化后的文本估算尺寸。
    ///
    /// 拉丁文字宽度按每字符 ≈ 0.55 × font_size 估算，
    /// 行高按 1.4 × font_size 估算。
    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let text = format_date(state);
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
        let text = format_date(state);
        if text.is_empty() {
            return;
        }
        let font_size: f32 = 16.0;
        let color = Color::new(0.1, 0.1, 0.1, 1.0); // 近黑色文字
        ctx.draw_text(&text, bounds, color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let text = format_date(state);
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

    // ── parse_iso_date 单元测试 ──

    #[test]
    fn parse_valid_date_only() {
        let parts = parse_iso_date("2026-06-20");
        assert!(parts.valid);
        assert_eq!(parts.year, 2026);
        assert_eq!(parts.month, 6);
        assert_eq!(parts.day, 20);
        assert_eq!(parts.hour, 0);
        assert_eq!(parts.minute, 0);
        assert_eq!(parts.second, 0);
    }

    #[test]
    fn parse_valid_datetime() {
        let parts = parse_iso_date("2026-06-20T14:30:45");
        assert!(parts.valid);
        assert_eq!(parts.year, 2026);
        assert_eq!(parts.month, 6);
        assert_eq!(parts.day, 20);
        assert_eq!(parts.hour, 14);
        assert_eq!(parts.minute, 30);
        assert_eq!(parts.second, 45);
    }

    #[test]
    fn parse_empty_string() {
        let parts = parse_iso_date("");
        assert!(!parts.valid);
    }

    #[test]
    fn parse_invalid_date() {
        let parts = parse_iso_date("not-a-date");
        assert!(!parts.valid);
    }

    #[test]
    fn parse_short_string() {
        let parts = parse_iso_date("2026");
        assert!(!parts.valid);
    }

    // ── day_of_week 单元测试 ──

    #[test]
    fn dow_known_date() {
        // 2026-06-20 is a Saturday
        assert_eq!(day_of_week(2026, 6, 20), 6); // 6 = Saturday
        // 2026-01-01 is a Thursday
        assert_eq!(day_of_week(2026, 1, 1), 4); // 4 = Thursday
        // 2024-02-29 (leap year) is a Thursday
        assert_eq!(day_of_week(2024, 2, 29), 4);
    }

    // ── format_date 单元测试 ──

    #[test]
    fn format_invalid_date_returns_empty() {
        let state = WaFormatDateState::new(); // date is empty
        assert_eq!(format_date(&state), "");
    }

    #[test]
    fn format_date_long_month_day_year() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.month = "long".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        assert_eq!(format_date(&state), "June 20, 2026");
    }

    #[test]
    fn format_date_short_month() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.month = "short".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        assert_eq!(format_date(&state), "Jun 20, 2026");
    }

    #[test]
    fn format_date_numeric_month() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.month = "numeric".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        assert_eq!(format_date(&state), "6 20, 2026");
    }

    #[test]
    fn format_date_2digit_month_day() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-03-05".into();
        state.month = "2-digit".into();
        state.day = "2-digit".into();
        state.year = "2-digit".into();
        assert_eq!(format_date(&state), "03 05, 26");
    }

    #[test]
    fn format_date_with_weekday() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.weekday = "long".into();
        state.month = "long".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        // 2026-06-20 is Saturday
        assert_eq!(format_date(&state), "Saturday, June 20, 2026");
    }

    #[test]
    fn format_date_short_weekday() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.weekday = "short".into();
        state.month = "short".into();
        state.day = "numeric".into();
        assert_eq!(format_date(&state), "Sat, Jun 20");
    }

    #[test]
    fn format_date_narrow_weekday() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.weekday = "narrow".into();
        state.month = "short".into();
        state.day = "numeric".into();
        assert_eq!(format_date(&state), "S, Jun 20");
    }

    #[test]
    fn format_time_24h() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20T14:30:45".into();
        state.hour = "2-digit".into();
        state.minute = "2-digit".into();
        state.second = "2-digit".into();
        assert_eq!(format_date(&state), "14:30:45");
    }

    #[test]
    fn format_time_12h() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20T14:30:45".into();
        state.hour = "numeric".into();
        state.minute = "2-digit".into();
        state.hour_format = "12".into();
        assert_eq!(format_date(&state), "2:30 PM");
    }

    #[test]
    fn format_time_12h_midnight() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20T00:05:00".into();
        state.hour = "numeric".into();
        state.minute = "2-digit".into();
        state.hour_format = "12".into();
        assert_eq!(format_date(&state), "12:05 AM");
    }

    #[test]
    fn format_time_12h_noon() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20T12:30:00".into();
        state.hour = "numeric".into();
        state.minute = "2-digit".into();
        state.hour_format = "12".into();
        assert_eq!(format_date(&state), "12:30 PM");
    }

    #[test]
    fn format_full_datetime() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20T14:30:45".into();
        state.month = "long".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        state.hour = "2-digit".into();
        state.minute = "2-digit".into();
        assert_eq!(format_date(&state), "June 20, 2026, 14:30");
    }

    // ── WidgetSpec 测试 ──

    #[test]
    fn name() {
        assert_eq!(WaFormatDate.name(), "rgui_components::WaFormatDate");
    }

    #[test]
    fn default_state() {
        let state = WaFormatDateState::new();
        assert_eq!(state.date, "");
        assert_eq!(state.hour_format, "auto");
    }

    #[test]
    fn view_has_props() {
        let state = WaFormatDateState::new();
        let v = WaFormatDate.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaFormatDate");
        assert!(v.props.contains_key("date"));
        assert!(v.props.contains_key("month"));
        assert!(v.props.contains_key("day"));
        assert!(v.props.contains_key("year"));
    }

    #[test]
    fn view_date_prop() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        let v = WaFormatDate.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("date").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "2026-06-20"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_month_prop() {
        let mut state = WaFormatDateState::new();
        state.month = "long".into();
        let v = WaFormatDate.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("month").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "long"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaFormatDateState::new();
        WaFormatDate.update(
            WaFormatDateMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_non_zero_for_valid_date() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.month = "long".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        let size = WaFormatDate.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width > 0.0, "宽度应 > 0，实际 {size:?}");
        assert!(size.height > 0.0, "高度应 > 0，实际 {size:?}");
    }

    #[test]
    fn measure_invalid_date_returns_zero() {
        let state = WaFormatDateState::new(); // empty date
        let size = WaFormatDate.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn measure_clamped_by_constraints() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.month = "long".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        let size = WaFormatDate.measure(
            &state,
            BoxConstraints::new(0.0, 10.0, 0.0, 10.0),
            &MeasureContext::default(),
        );
        assert!(size.width <= 10.0, "宽度应被约束限制，实际 {size:?}");
        assert!(size.height <= 10.0, "高度应被约束限制，实际 {size:?}");
    }

    #[test]
    fn paint_produces_ops() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.month = "long".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaFormatDate.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应产生至少 1 个绘制操作");
    }

    #[test]
    fn paint_invalid_date_produces_no_ops() {
        let state = WaFormatDateState::new();
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaFormatDate.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "无效日期不应产生绘制操作");
    }

    #[test]
    fn accessibility_has_label() {
        let mut state = WaFormatDateState::new();
        state.date = "2026-06-20".into();
        state.month = "long".into();
        state.day = "numeric".into();
        state.year = "numeric".into();
        let node = WaFormatDate.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_some(), "应为格式化文本提供标签");
    }

    #[test]
    fn accessibility_invalid_no_label() {
        let state = WaFormatDateState::new();
        let node = WaFormatDate.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_none());
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaFormatDateMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaFormatDateState::schema_name(), "WaFormatDateState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaFormatDateState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaFormatDateState>());
    }
}
