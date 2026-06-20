/// Translated from Web Awesome wa-textarea
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

/// Web Awesome wa-textarea 组件状态。
///
/// Textarea 是多行文本输入框，支持标签、提示、验证、
/// 字符计数和尺寸调整。
///
/// 跳过 withLabel/withHint SSR 属性。
/// 跳过 autocapitalize/autocorrect/autocomplete/autofocus/enterkeyhint
/// 等 Web 专属属性。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTextareaState {
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 视觉外观：filled | outlined | filled-outlined
    pub appearance: String,
    /// 标签文本
    pub label: String,
    /// 提示文本
    pub hint: String,
    /// 占位符文本
    pub placeholder: String,
    /// 当前值（多行文本）
    pub value: String,
    /// 默认值（表单重置时使用）
    pub default_value: String,
    /// 表单 name 属性
    pub name: String,
    /// 调整大小方式：none | vertical | horizontal | both | auto
    pub resize: String,
    /// 虚拟键盘输入模式
    pub inputmode: String,
    /// 默认显示行数
    pub rows: u32,
    /// 禁用状态
    pub disabled: bool,
    /// 只读状态
    pub readonly: bool,
    /// 必填字段
    pub required: bool,
    /// 拼写检查
    pub spellcheck: bool,
    /// 显示字符计数
    pub with_count: bool,
    /// 最小输入长度
    pub minlength: Option<u32>,
    /// 最大输入长度
    pub maxlength: Option<u32>,
}

impl WaTextareaState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: "m".into(),
            appearance: "outlined".into(),
            resize: "vertical".into(),
            rows: 4,
            spellcheck: true,
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTextareaMessage {
    /// 失去焦点
    Blur,
    /// 值已提交
    Change,
    /// 获得焦点
    Focus,
    /// 接收输入
    Input,
    /// 验证失败
    Invalid,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaTextarea;

/// 根据 size 属性的像素映射
fn textarea_height_scale(size: &str) -> f64 {
    match size {
        "xs" => 16.0,
        "s" | "small" => 18.0,
        "l" | "large" => 28.0,
        "xl" => 32.0,
        _ => 22.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的字体大小映射
fn textarea_font_size(size: &str) -> f64 {
    match size {
        "xs" => 12.0,
        "s" | "small" => 14.0,
        "l" | "large" => 20.0,
        "xl" => 24.0,
        _ => 16.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的 border-radius 映射
fn textarea_border_radius(size: &str) -> f32 {
    match size {
        "xs" => 2.0,
        "s" | "small" => 3.0,
        "l" | "large" => 6.0,
        "xl" => 8.0,
        _ => 4.0, // "m" | "medium" (默认)
    }
}

impl WidgetSpec for WaTextarea {
    type State = WaTextareaState;
    type Message = WaTextareaMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTextarea"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaTextarea")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("value", PropValue::str(state.value.as_str()))
            .prop("placeholder", PropValue::str(state.placeholder.as_str()))
            .prop("hint", PropValue::str(state.hint.as_str()))
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("resize", PropValue::str(state.resize.as_str()))
            .prop("inputmode", PropValue::str(state.inputmode.as_str()))
            .prop("rows", PropValue::Int(state.rows as i64))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("readonly", PropValue::Bool(state.readonly))
            .prop("required", PropValue::Bool(state.required))
            .prop("spellcheck", PropValue::Bool(state.spellcheck))
            .prop("with-count", PropValue::Bool(state.with_count));
        if let Some(min) = state.minlength {
            v = v.prop("minlength", PropValue::Int(min as i64));
        }
        if let Some(max) = state.maxlength {
            v = v.prop("maxlength", PropValue::Int(max as i64));
        }
        v
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTextareaMessage::Change => {
                // 值提交——由 IME/框架事件系统处理后调用
            }
            WaTextareaMessage::Input => {
                // 输入中——由 IME 驱动，state.value 由框架更新
            }
            WaTextareaMessage::Blur
            | WaTextareaMessage::Focus
            | WaTextareaMessage::Invalid => {
                // 这些事件由框架事件系统处理
            }
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let line_h = textarea_height_scale(&state.size);
        let font_size = textarea_font_size(&state.size);
        let rows = state.rows.max(2) as f64;

        // 文本区域高度 = 行数 × 行高 + 内边距
        let textarea_h = rows * line_h + 16.0; // 16px 垂直 padding

        // 标签高度
        let label_h = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };

        // 提示/计数区域高度
        let footer_h = if state.hint.is_empty() && !state.with_count {
            0.0
        } else {
            font_size * 0.85 * 1.5
        };

        let total_h = label_h + textarea_h + footer_h;

        // 宽度：根据内容估算
        let char_count = 30; // 多行文本固定宽度
        let text_width = char_count as f64 * font_size * 0.6 + 24.0;

        Size::new(
            text_width.clamp(c.min_width, c.max_width),
            total_h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let line_h = textarea_height_scale(&state.size);
        let font_size = textarea_font_size(&state.size);
        let border_radius = textarea_border_radius(&state.size);
        let rows = state.rows.max(2) as f64;

        // 标签绘制偏移
        let label_offset_y = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };
        let textarea_y = bounds.origin.y + label_offset_y;
        let textarea_h = rows * line_h + 16.0;
        let textarea_rect = Rect::new(bounds.origin.x, textarea_y, bounds.size.width, textarea_h);

        // ── 标签 ──
        if !state.label.is_empty() {
            let label_font_size = font_size as f32 * 0.9;
            let label_color = if state.disabled {
                Color::new(0.6, 0.6, 0.6, 1.0)
            } else {
                Color::new(0.2, 0.2, 0.2, 1.0)
            };
            let label_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                label_offset_y,
            );
            ctx.draw_text(&state.label, label_rect, label_color, label_font_size);
        }

        // ── 文本区域背景 ──
        let (bg, border_color) = match state.appearance.as_str() {
            "filled" => (
                Color::new(0.95, 0.95, 0.95, 1.0),
                Color::new(0.85, 0.85, 0.85, 1.0),
            ),
            "filled-outlined" => (
                Color::new(0.95, 0.95, 0.95, 1.0),
                Color::new(0.6, 0.6, 0.6, 1.0),
            ),
            _ => (Color::WHITE, Color::new(0.6, 0.6, 0.6, 1.0)), // outlined (默认)
        };

        let bg_color = if state.disabled {
            Color::new(0.92, 0.92, 0.92, 0.5)
        } else {
            bg
        };
        let border = if state.disabled {
            Color::new(0.7, 0.7, 0.7, 0.5)
        } else {
            border_color
        };

        // 绘制背景（带 border-radius）
        ctx.fill_rect(textarea_rect, bg_color, border_radius);

        // 模拟边框
        let border_width: f64 = 2.0;
        ctx.fill_rect(textarea_rect, border, border_radius);
        let inner_rect = Rect::new(
            textarea_rect.origin.x + border_width,
            textarea_rect.origin.y + border_width,
            textarea_rect.size.width - border_width * 2.0,
            textarea_rect.size.height - border_width * 2.0,
        );
        let inner_radius = (border_radius as f64 - 1.0).max(1.0) as f32;
        ctx.fill_rect(inner_rect, bg_color, inner_radius);

        // ── 值文本 / 占位符 ──
        let display_text = if state.value.is_empty() {
            &state.placeholder
        } else {
            &state.value
        };

        let text_color = if state.value.is_empty() || state.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0) // 占位符或禁用颜色
        } else {
            Color::new(0.1, 0.1, 0.1, 1.0)
        };

        if !display_text.is_empty() {
            let text_padding: f64 = 8.0;
            // 多行文本：从上到下绘制多行
            let text_lines: Vec<&str> = display_text.lines().collect();
            let line_height = line_h;

            for (i, line) in text_lines.iter().enumerate() {
                if i as f64 * line_height > textarea_h - text_padding * 2.0 {
                    break; // 超出可见区域
                }
                let line_rect = Rect::new(
                    inner_rect.origin.x + text_padding,
                    inner_rect.origin.y + text_padding + i as f64 * line_height,
                    inner_rect.size.width - text_padding * 2.0,
                    line_height,
                );
                ctx.draw_text(line, line_rect, text_color, font_size as f32);
            }
        }

        // ── 提示文本 ──
        if !state.hint.is_empty() {
            let hint_y = textarea_rect.origin.y + textarea_rect.size.height + 4.0;
            let hint_font_size = (font_size * 0.75) as f32;
            let hint_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let hint_rect = Rect::new(bounds.origin.x, hint_y, bounds.size.width, font_size * 0.75);
            ctx.draw_text(&state.hint, hint_rect, hint_color, hint_font_size);
        }

        // ── 字符计数 ──
        if state.with_count {
            let count_y = if state.hint.is_empty() {
                textarea_rect.origin.y + textarea_rect.size.height + 4.0
            } else {
                textarea_rect.origin.y + textarea_rect.size.height + 4.0 + font_size * 0.75
            };
            let count_font_size = (font_size * 0.7) as f32;
            let count_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let count_text = match state.maxlength {
                Some(max) => {
                    let current = state.value.chars().count() as u32;
                    format!("{}/{}", current, max)
                }
                None => {
                    let current = state.value.chars().count();
                    format!("{}", current)
                }
            };
            let count_rect = Rect::new(
                bounds.origin.x + bounds.size.width - 60.0,
                count_y,
                60.0,
                font_size * 0.7,
            );
            ctx.draw_text(&count_text, count_rect, count_color, count_font_size);
        }
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
        assert_eq!(WaTextarea.name(), "rgui_components::WaTextarea");
    }

    #[test]
    fn view_has_label() {
        let state = WaTextareaState::new("Description");
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_size() {
        let mut state = WaTextareaState::new("Notes");
        state.size = "l".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("size"),
            Some(&PropValue::Str(std::sync::Arc::from("l")))
        );
    }

    #[test]
    fn view_has_appearance() {
        let mut state = WaTextareaState::new("Bio");
        state.appearance = "filled".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("appearance"),
            Some(&PropValue::Str(std::sync::Arc::from("filled")))
        );
    }

    #[test]
    fn view_has_value() {
        let mut state = WaTextareaState::new("Comment");
        state.value = "Hello\nWorld".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("value"),
            Some(&PropValue::Str(std::sync::Arc::from("Hello\nWorld")))
        );
    }

    #[test]
    fn view_has_placeholder() {
        let mut state = WaTextareaState::new("Message");
        state.placeholder = "Enter your message...".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("placeholder"),
            Some(&PropValue::Str(std::sync::Arc::from(
                "Enter your message..."
            )))
        );
    }

    #[test]
    fn view_has_hint() {
        let mut state = WaTextareaState::new("Feedback");
        state.hint = "Max 500 characters.".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("hint"),
            Some(&PropValue::Str(std::sync::Arc::from(
                "Max 500 characters."
            )))
        );
    }

    #[test]
    fn view_has_resize() {
        let mut state = WaTextareaState::new("Resizable");
        state.resize = "none".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("resize"),
            Some(&PropValue::Str(std::sync::Arc::from("none")))
        );
    }

    #[test]
    fn view_has_rows() {
        let mut state = WaTextareaState::new("Tall");
        state.rows = 8;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("rows"), Some(&PropValue::Int(8)));
    }

    #[test]
    fn view_disabled() {
        let mut state = WaTextareaState::new("Locked");
        state.disabled = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_readonly() {
        let mut state = WaTextareaState::new("ReadOnly");
        state.readonly = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("readonly"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_required() {
        let mut state = WaTextareaState::new("Must fill");
        state.required = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("required"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_spellcheck() {
        let mut state = WaTextareaState::new("Spell");
        state.spellcheck = false;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("spellcheck"), Some(&PropValue::Bool(false)));
    }

    #[test]
    fn view_with_count() {
        let mut state = WaTextareaState::new("Counted");
        state.with_count = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("with-count"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_has_minlength() {
        let mut state = WaTextareaState::new("Min");
        state.minlength = Some(10);
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("minlength"), Some(&PropValue::Int(10)));
    }

    #[test]
    fn view_has_maxlength() {
        let mut state = WaTextareaState::new("Max");
        state.maxlength = Some(500);
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("maxlength"), Some(&PropValue::Int(500)));
    }

    #[test]
    fn view_no_minlength_when_none() {
        let state = WaTextareaState::new("No min");
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(!v.props.contains_key("minlength"));
    }

    #[test]
    fn view_no_maxlength_when_none() {
        let state = WaTextareaState::new("No max");
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(!v.props.contains_key("maxlength"));
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_change_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_focus_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Focus,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_input_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Input,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_invalid_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Invalid,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_min_constraints() {
        let state = WaTextareaState::new("Group");
        let size = WaTextarea.measure(
            &state,
            BoxConstraints::new(200.0, 800.0, 100.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 200.0);
        assert!(size.height >= 100.0);
    }

    #[test]
    fn measure_with_label_is_taller() {
        let state_no_label = WaTextareaState::new("");
        let state_with_label = WaTextareaState::new("Description");

        let size_no = WaTextarea.measure(
            &state_no_label,
            BoxConstraints::new(200.0, 800.0, 100.0, 600.0),
            &MeasureContext::default(),
        );
        let size_with = WaTextarea.measure(
            &state_with_label,
            BoxConstraints::new(200.0, 800.0, 100.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size_with.height > size_no.height);
    }

    #[test]
    fn paint_with_label_produces_ops() {
        let state = WaTextareaState::new("My Textarea");
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制标签文本");
    }

    #[test]
    fn paint_with_value_produces_ops() {
        let mut state = WaTextareaState::new("Content");
        state.value = "Line 1\nLine 2\nLine 3".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制文本内容");
    }

    #[test]
    fn paint_with_placeholder_produces_ops() {
        let mut state = WaTextareaState::new("Placeholder");
        state.placeholder = "Type something...".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制占位符文本");
    }

    #[test]
    fn paint_with_hint_produces_ops() {
        let mut state = WaTextareaState::new("Group");
        state.hint = "Enter multiple lines.".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制提示文本");
    }

    #[test]
    fn paint_empty_no_label_no_ops() {
        let state = WaTextareaState::new("");
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        // 即使无 label，仍绘制背景边框，所以应有 ops
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_with_count() {
        let mut state = WaTextareaState::new("Counted");
        state.with_count = true;
        state.value = "Test".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_with_count_and_maxlength() {
        let mut state = WaTextareaState::new("Limited");
        state.with_count = true;
        state.maxlength = Some(500);
        state.value = "Hello".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn accessibility_has_label() {
        let state = WaTextareaState::new("Accessibility Textarea");
        let node = WaTextarea.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Accessibility Textarea"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaTextareaMessage::Change.message_name(), "change");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaTextareaState::schema_name(), "WaTextareaState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaTextareaState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaTextareaState>());
    }

    #[test]
    fn default_state_values() {
        let state = WaTextareaState::default();
        assert!(state.label.is_empty());
        assert!(!state.disabled);
        assert!(!state.readonly);
        assert!(!state.required);
        assert_eq!(state.resize, "");
    }

    #[test]
    fn new_state_has_defaults() {
        let state = WaTextareaState::new("Title");
        assert_eq!(state.label, "Title");
        assert_eq!(state.size, "m");
        assert_eq!(state.appearance, "outlined");
        assert_eq!(state.resize, "vertical");
        assert_eq!(state.rows, 4);
        assert!(state.spellcheck);
    }
}
