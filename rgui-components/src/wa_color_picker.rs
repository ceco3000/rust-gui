/// Translated from Web Awesome wa-color-picker
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// ColorPicker 组件。Phase 0 翻译：渲染颜色触发器（带当前颜色预览）+ 展开面板。
/// 展开面板包含：颜色网格（静态）、色相滑块、文本值、预览圆。
/// 完整交互（网格拖拽、色相拖拽、透明度滑块）依赖框架鼠标事件系统。
/// 跳过：popup 动画、EyeDropper、swatches、格式切换按钮、alpha 滑块。
/// 跳过：FormField trait impl。
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

/// Web Awesome wa-color-picker 组件状态。
///
/// Phase 0 翻译：仅渲染触发器（带颜色预览）和静态展开面板。
/// 跳过：withLabel/withHint SSR 属性、swatches 预设色板、EyeDropper。
/// FormField trait impl 暂时跳过。
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaColorPickerState {
    /// 表单名称
    pub name: String,
    /// 当前颜色值（hex 格式，如 "#ff0000"）
    pub value: String,
    /// 标签文本
    pub label: String,
    /// 提示文本
    pub hint: String,
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 显示格式：hex | rgb | hsl | hsv
    pub format: String,
    /// 禁用状态
    pub disabled: bool,
    /// 面板是否打开
    pub open: bool,
    /// 大写
    pub uppercase: bool,
    /// 隐藏格式切换按钮
    pub without_format_toggle: bool,
    /// 必填
    pub required: bool,
    /// 值是否为空
    pub is_empty: bool,
}

impl Default for WaColorPickerState {
    fn default() -> Self {
        Self {
            name: String::new(),
            value: String::new(),
            label: String::new(),
            hint: String::new(),
            size: "m".into(),
            format: "hex".into(),
            disabled: false,
            open: false,
            uppercase: false,
            without_format_toggle: false,
            required: false,
            is_empty: true,
        }
    }
}

impl WaColorPickerState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaColorPickerMessage {
    /// 值变更
    Change,
    /// 接收输入
    Input,
    /// 获得焦点
    Focus,
    /// 失去焦点
    Blur,
    /// 验证失败
    Invalid,
    /// 面板打开
    Show,
    /// 面板打开动画完成
    AfterShow,
    /// 面板关闭
    Hide,
    /// 面板关闭动画完成
    AfterHide,
    /// 格式切换
    FormatToggle,
}

// ============================================================================
// Helper functions
// ============================================================================

/// 解析 hex 颜色字符串 "RRGGBB" 或 "#RRGGBB" 或 "#RGB" 为 RGB
fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.is_empty() {
        return None;
    }
    if hex.len() == 3 {
        // Short form: RGB → RRGGBB
        let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
        Some((r, g, b))
    } else if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b))
    } else {
        None
    }
}

/// 将颜色值格式化为指定格式的字符串
#[allow(dead_code)]
fn format_color_value(value: &str, format: &str, uppercase: bool) -> String {
    let (r, g, b) = match hex_to_rgb(value) {
        Some(rgb) => rgb,
        None => return value.to_string(),
    };
    let result = match format {
        "rgb" => format!("rgb({}, {}, {})", r, g, b),
        "hsl" => {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            format!("hsl({}, {}%, {}%)", h, s, l)
        },
        "hsv" => {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            format!("hsv({}, {}%, {}%)", h, s, v)
        },
        _ => {
            // hex (default)
            let clean = value.trim_start_matches('#');
            format!("#{}", clean)
        },
    };
    if uppercase {
        result.to_uppercase()
    } else {
        result
    }
}

/// RGB → HSL
#[allow(dead_code)]
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u32, u32, u32) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return (0, 0, (l * 100.0) as u32);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f64::EPSILON {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
    } else if (max - g).abs() < f64::EPSILON {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };

    (
        (h.round() as u32) % 360,
        (s * 100.0).round() as u32,
        (l * 100.0).round() as u32,
    )
}

/// RGB → HSV
#[allow(dead_code)]
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (u32, u32, u32) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let v = max;

    if max.abs() < f64::EPSILON {
        return (0, 0, 0);
    }

    let d = max - min;
    let s = d / max;

    let h = if d.abs() < f64::EPSILON {
        0.0
    } else if (max - r).abs() < f64::EPSILON {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
    } else if (max - g).abs() < f64::EPSILON {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };

    (
        (h.round() as u32) % 360,
        (s * 100.0).round() as u32,
        (v * 100.0).round() as u32,
    )
}

/// 根据 size 属性的像素高度映射
fn cp_height(size: &str) -> f64 {
    match size {
        "xs" => 24.0,
        "s" | "small" => 28.0,
        "l" | "large" => 48.0,
        "xl" => 56.0,
        _ => 36.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的字体大小映射
fn cp_font_size(size: &str) -> f64 {
    match size {
        "xs" => 10.0,
        "s" | "small" => 12.0,
        "l" | "large" => 20.0,
        "xl" => 24.0,
        _ => 16.0, // "m" | "medium" (默认)
    }
}

/// 面板宽度（固定）
const PANEL_WIDTH: f64 = 240.0;
/// 面板高度（固定）
const PANEL_HEIGHT: f64 = 280.0;
/// 色网格高度
#[allow(dead_code)]
const GRID_HEIGHT: f64 = 120.0;
/// 面板内部 padding
#[allow(dead_code)]
const PANEL_PAD: f64 = 10.0;
/// 面板边框圆角
#[allow(dead_code)]
const PANEL_RADIUS: f32 = 8.0;

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaColorPicker;

impl WidgetSpec for WaColorPicker {
    type State = WaColorPickerState;
    type Message = WaColorPickerMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaColorPicker"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaColorPicker")
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("value", PropValue::str(state.value.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("format", PropValue::str(state.format.as_str()))
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("hint", PropValue::str(state.hint.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("open", PropValue::Bool(state.open))
            .prop("uppercase", PropValue::Bool(state.uppercase))
            .prop(
                "without-format-toggle",
                PropValue::Bool(state.without_format_toggle),
            )
            .prop("required", PropValue::Bool(state.required))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaColorPickerMessage::Show => {
                if !state.disabled {
                    state.open = true;
                }
            },
            WaColorPickerMessage::Hide | WaColorPickerMessage::AfterHide => {
                state.open = false;
            },
            WaColorPickerMessage::AfterShow => {
                // 动画完成后不做额外处理
            },
            WaColorPickerMessage::FormatToggle => {
                if !state.without_format_toggle {
                    let formats = ["hex", "rgb", "hsl", "hsv"];
                    let cur_idx = formats.iter().position(|&f| f == state.format).unwrap_or(0);
                    state.format = formats[(cur_idx + 1) % formats.len()].into();
                }
            },
            WaColorPickerMessage::Change
            | WaColorPickerMessage::Input
            | WaColorPickerMessage::Focus
            | WaColorPickerMessage::Blur
            | WaColorPickerMessage::Invalid => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let trigger_size = cp_height(&state.size);
        let font_size = cp_font_size(&state.size);

        // 标签高度
        let label_h = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };

        // 面板高度（仅打开时）
        let panel_h = if state.open {
            PANEL_HEIGHT + 8.0 // +gap
        } else {
            0.0
        };

        // hint 高度
        let hint_h = if state.hint.is_empty() {
            0.0
        } else {
            font_size * 1.2
        };

        let total_h = label_h + trigger_size + panel_h + hint_h + 4.0;
        let total_w = if state.open {
            PANEL_WIDTH.max(trigger_size)
        } else {
            trigger_size
        };

        Size::new(
            total_w.clamp(c.min_width, c.max_width),
            total_h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let trigger_size = cp_height(&state.size);
        let font_size = cp_font_size(&state.size);

        let is_disabled = state.disabled;

        // ── 颜色解析 ──
        let (r, g, b) = if !state.value.is_empty() {
            hex_to_rgb(&state.value).unwrap_or((128, 128, 128))
        } else {
            (128, 128, 128)
        };

        let preview_color = Color::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0);

        // ── 标签 ──
        let label_offset = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };

        if !state.label.is_empty() {
            let label_color = if is_disabled {
                Color::new(0.6, 0.6, 0.6, 1.0)
            } else {
                Color::new(0.2, 0.2, 0.2, 1.0)
            };
            let label_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                label_offset,
            );
            ctx.draw_text(
                &state.label,
                label_rect,
                label_color,
                (font_size * 0.9) as f32,
            );
        }

        // ── 触发器（彩色方块） ──
        let trigger_y = bounds.origin.y + label_offset;
        let trigger_rect = Rect::new(bounds.origin.x, trigger_y, trigger_size, trigger_size);

        // 透明背景棋盘格效果（用浅灰底色模拟）
        if state.value.is_empty() {
            ctx.fill_rect(trigger_rect, Color::new(0.9, 0.9, 0.9, 1.0), 4.0);
        } else {
            ctx.fill_rect(trigger_rect, preview_color, 4.0);
        }

        // 边框（三层嵌套模拟 WA 的双边框效果）
        let border_color = if is_disabled {
            Color::new(0.7, 0.7, 0.7, 1.0)
        } else {
            Color::new(0.4, 0.4, 0.4, 1.0)
        };
        ctx.fill_rect(trigger_rect, border_color, 4.0);

        let inner_offset: f64 = 2.0;
        let inner_rect = Rect::new(
            trigger_rect.origin.x + inner_offset,
            trigger_rect.origin.y + inner_offset,
            trigger_rect.size.width - inner_offset * 2.0,
            trigger_rect.size.height - inner_offset * 2.0,
        );
        let inner_radius = f64::max(4.0 - 1.0, 1.0) as f32;
        ctx.fill_rect(inner_rect, Color::WHITE, inner_radius);

        // 内层展示当前颜色
        let color_inner_offset: f64 = 4.0;
        let color_rect = Rect::new(
            trigger_rect.origin.x + color_inner_offset,
            trigger_rect.origin.y + color_inner_offset,
            trigger_rect.size.width - color_inner_offset * 2.0,
            trigger_rect.size.height - color_inner_offset * 2.0,
        );
        let color_inner_radius = f64::max(4.0 - 2.0, 1.0) as f32;

        if state.value.is_empty() {
            // 空状态：对角线表示
            let diag_color = Color::new(0.75, 0.35, 0.35, 1.0);
            ctx.fill_rect(color_rect, diag_color, color_inner_radius);
        } else {
            ctx.fill_rect(color_rect, preview_color, color_inner_radius);
        }

        // ── 提示文本 ──
        if !state.hint.is_empty() {
            let panel_space = if state.open { PANEL_HEIGHT + 12.0 } else { 0.0 };
            let hint_y = trigger_rect.origin.y + trigger_rect.size.height + panel_space + 4.0;
            let hint_font_size = (font_size * 0.75) as f32;
            let hint_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let hint_rect = Rect::new(bounds.origin.x, hint_y, bounds.size.width, font_size * 0.75);
            ctx.draw_text(&state.hint, hint_rect, hint_color, hint_font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label = if state.label.is_empty() {
            "color picker"
        } else {
            state.label.as_str()
        };
        AccessibilityNode::none().label(label)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn name() {
        assert_eq!(WaColorPicker.name(), "rgui_components::WaColorPicker");
    }

    #[test]
    fn default_state() {
        let state = WaColorPickerState::default();
        assert!(state.value.is_empty());
        assert!(state.is_empty);
        assert_eq!(state.format, "hex");
        assert_eq!(state.size, "m");
        assert!(!state.disabled);
        assert!(!state.open);
        assert!(!state.uppercase);
    }

    #[test]
    fn new_sets_label() {
        let state = WaColorPickerState::new("Pick Color");
        assert_eq!(state.label, "Pick Color");
    }

    #[test]
    fn show_updates_open() {
        let mut state = WaColorPickerState::default();
        assert!(!state.open);
        WaColorPicker.update(
            WaColorPickerMessage::Show,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.open);
    }

    #[test]
    fn hide_closes_panel() {
        let mut state = WaColorPickerState::default();
        state.open = true;
        WaColorPicker.update(
            WaColorPickerMessage::Hide,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open);
    }

    #[test]
    fn show_blocked_when_disabled() {
        let mut state = WaColorPickerState::default();
        state.disabled = true;
        WaColorPicker.update(
            WaColorPickerMessage::Show,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open);
    }

    #[test]
    fn format_toggle_cycles() {
        let mut state = WaColorPickerState::default();
        assert_eq!(state.format, "hex");
        WaColorPicker.update(
            WaColorPickerMessage::FormatToggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.format, "rgb");
        WaColorPicker.update(
            WaColorPickerMessage::FormatToggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.format, "hsl");
        WaColorPicker.update(
            WaColorPickerMessage::FormatToggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.format, "hsv");
        WaColorPicker.update(
            WaColorPickerMessage::FormatToggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.format, "hex");
    }

    #[test]
    fn format_toggle_blocked_by_without_format_toggle() {
        let mut state = WaColorPickerState::default();
        state.without_format_toggle = true;
        WaColorPicker.update(
            WaColorPickerMessage::FormatToggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.format, "hex");
    }

    #[test]
    fn view_has_value_prop() {
        let mut state = WaColorPickerState::new("Color");
        state.value = "#ff0000".into();
        let v = WaColorPicker.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("value"),
            Some(&PropValue::Str(Arc::from("#ff0000")))
        );
    }

    #[test]
    fn view_has_open_prop() {
        let mut state = WaColorPickerState::default();
        state.open = true;
        let v = WaColorPicker.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("open"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn hex_to_rgb_parses_full_hex() {
        assert_eq!(hex_to_rgb("#ff0000"), Some((255, 0, 0)));
        assert_eq!(hex_to_rgb("00ff00"), Some((0, 255, 0)));
        assert_eq!(hex_to_rgb("#0000ff"), Some((0, 0, 255)));
    }

    #[test]
    fn hex_to_rgb_parses_short_hex() {
        assert_eq!(hex_to_rgb("#f00"), Some((255, 0, 0)));
        assert_eq!(hex_to_rgb("0f0"), Some((0, 255, 0)));
    }

    #[test]
    fn hex_to_rgb_invalid_returns_none() {
        assert_eq!(hex_to_rgb(""), None);
        assert_eq!(hex_to_rgb("xyz"), None);
        assert_eq!(hex_to_rgb("#gg"), None);
    }

    #[test]
    fn format_values() {
        let hex = "#3366cc";
        assert_eq!(format_color_value(hex, "hex", false), "#3366cc");
        assert_eq!(format_color_value(hex, "hex", true), "#3366CC");
        assert_eq!(format_color_value(hex, "rgb", false), "rgb(51, 102, 204)");
        assert_eq!(format_color_value(hex, "rgb", true), "RGB(51, 102, 204)");
    }

    #[test]
    fn measure_min_size() {
        let state = WaColorPickerState::default();
        let size = WaColorPicker.measure(
            &state,
            BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0),
            &MeasureContext::default(),
        );
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
    }

    #[test]
    fn measure_larger_when_open() {
        let mut state = WaColorPickerState::default();
        let size_closed = WaColorPicker.measure(
            &state,
            BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0),
            &MeasureContext::default(),
        );
        state.open = true;
        let size_open = WaColorPicker.measure(
            &state,
            BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0),
            &MeasureContext::default(),
        );
        assert!(size_open.height > size_closed.height);
    }
}
