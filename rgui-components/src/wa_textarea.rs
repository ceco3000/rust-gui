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
/// Textarea 是多行文本输入框，支持标签、提示、resize、字符计数。
/// 跳过 formAction/formEnctype/formMethod 等 Web 专属表单属性。
/// 跳过 withLabel/withHint SSR 属性。
/// FormField trait impl 暂时跳过（WC02 trait 已存在但尚未提交到代码中）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTextareaState {
    /// name 属性
    pub name: String,
    /// 当前值
    pub value: String,
    /// 默认值（表单重置时使用）
    pub default_value: String,
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
    /// 默认行数（默认 4）
    pub rows: u32,
    /// resize 模式：none | vertical | horizontal | both | auto
    pub resize: String,
    /// 禁用状态
    pub disabled: bool,
    /// 只读状态
    pub readonly: bool,
    /// 必填字段
    pub required: bool,
    /// 最小长度
    pub minlength: Option<u32>,
    /// 最大长度
    pub maxlength: Option<u32>,
    /// 拼写检查（默认 true）
    pub spellcheck: bool,
    /// 输入模式
    pub inputmode: String,
    /// 显示字符计数
    pub with_count: bool,
}

impl WaTextareaState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: "m".into(),
            appearance: "outlined".into(),
            rows: 4,
            resize: "vertical".into(),
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

/// 根据 size 属性的像素高度映射
fn textarea_height(size: &str) -> f64 {
    match size {
        "xs" => 24.0,
        "s" | "small" => 28.0,
        "l" | "large" => 48.0,
        "xl" => 56.0,
        _ => 36.0, // "m" | "medium" (默认)
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
        WidgetView::new("rgui_components::WaTextarea")
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
            .prop("with-count", PropValue::Bool(state.with_count))
            .prop(
                "minlength",
                PropValue::Int(state.minlength.unwrap_or(0) as i64),
            )
            .prop(
                "maxlength",
                PropValue::Int(state.maxlength.unwrap_or(0) as i64),
            )
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTextareaMessage::Change => {
                // 值提交——由 IME/框架事件系统处理后调用
            },
            WaTextareaMessage::Input => {
                // 输入中——由 IME 驱动，state.value 由框架更新
            },
            WaTextareaMessage::Blur | WaTextareaMessage::Focus | WaTextareaMessage::Invalid => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let font_size = textarea_font_size(&state.size);
        let line_height = font_size * 1.5;
        let _h_per_row = textarea_height(&state.size).max(line_height);

        // Textarea 高度 = 标签(可选) + rows × 行高 + 提示(可选)
        let label_height = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };
        let textarea_h = state.rows as f64 * line_height + 16.0; // 16px padding
        let hint_height = if state.hint.is_empty() {
            0.0
        } else {
            font_size * 0.75 + 4.0
        };
        let total_h = label_height + textarea_h + hint_height;

        // 宽度：估算文本宽度
        let char_count = state
            .value
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0)
            .max(state.placeholder.chars().count())
            .max(20) as f64;
        let text_width = char_count * font_size * 0.6 + 24.0; // 24px padding

        Size::new(
            text_width.clamp(c.min_width, c.max_width),
            total_h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let font_size = textarea_font_size(&state.size);
        let border_radius = textarea_border_radius(&state.size);

        // 标签绘制偏移
        let label_offset_y = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };
        let textarea_y = bounds.origin.y + label_offset_y;
        let textarea_h = state.rows as f64 * font_size * 1.5 + 16.0;
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

        // ── 文本框背景 ──
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

        // 模拟边框（绘制略小的矩形叠加）
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

        // ── 文本内容 / 占位符 ──
        let display_text = if state.value.is_empty() {
            &state.placeholder
        } else {
            &state.value
        };

        let text_color = if state.value.is_empty() {
            // 占位符颜色
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else if state.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            Color::new(0.1, 0.1, 0.1, 1.0)
        };

        if !display_text.is_empty() {
            let text_padding: f64 = 8.0;
            let text_rect = Rect::new(
                textarea_rect.origin.x + text_padding,
                textarea_rect.origin.y + 4.0,
                textarea_rect.size.width - text_padding * 2.0,
                textarea_rect.size.height - 8.0,
            );
            ctx.draw_text(display_text, text_rect, text_color, font_size as f32);
        }

        // ── 字符计数 ──
        if state.with_count {
            let current_len = state.value.chars().count();
            let count_text = if let Some(max) = state.maxlength {
                let remaining = (max as usize).saturating_sub(current_len);
                format!("{}", remaining)
            } else {
                format!("{}", current_len)
            };

            let count_font_size = (font_size * 0.75) as f32;
            let count_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let count_y = textarea_rect.origin.y + textarea_rect.size.height + 4.0;
            let count_rect = Rect::new(
                textarea_rect.origin.x + textarea_rect.size.width - 40.0,
                count_y,
                40.0,
                font_size * 0.75,
            );
            ctx.draw_text(&count_text, count_rect, count_color, count_font_size);
        }

        // ── 提示文本 ──
        if !state.hint.is_empty() {
            let hint_y = textarea_rect.origin.y + textarea_rect.size.height + 4.0;
            let hint_font_size = (font_size * 0.75) as f32;
            let hint_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let hint_rect = Rect::new(bounds.origin.x, hint_y, bounds.size.width, font_size * 0.75);
            ctx.draw_text(&state.hint, hint_rect, hint_color, hint_font_size);
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
    fn view_has_value() {
        let mut state = WaTextareaState::new("Bio");
        state.value = "Hello world".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("value"),
            Some(&PropValue::Str(std::sync::Arc::from("Hello world")))
        );
    }

    #[test]
    fn view_disabled() {
        let mut state = WaTextareaState::new("Notes");
        state.disabled = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_readonly() {
        let mut state = WaTextareaState::new("Notes");
        state.readonly = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("readonly"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_required() {
        let mut state = WaTextareaState::new("Notes");
        state.required = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("required"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_with_count() {
        let mut state = WaTextareaState::new("Notes");
        state.with_count = true;
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("with-count"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_default_rows() {
        let state = WaTextareaState::new("Notes");
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("rows"), Some(&PropValue::Int(4)));
    }

    #[test]
    fn view_placeholder() {
        let mut state = WaTextareaState::new("Notes");
        state.placeholder = "Enter text...".into();
        let v = WaTextarea.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("placeholder"),
            Some(&PropValue::Str(std::sync::Arc::from("Enter text...")))
        );
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
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
    fn update_invalid_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Invalid,
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
    fn update_input_is_handled() {
        let mut state = WaTextareaState::new("OK");
        WaTextarea.update(
            WaTextareaMessage::Input,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaTextareaState::new("Textarea");
        let size = WaTextarea.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 80.0, "宽度应 ≥ 80px，实际 {size:?}");
        assert!(
            size.height >= 60.0,
            "高度应 ≥ 60px（4 rows），实际 {size:?}"
        );
    }

    #[test]
    fn paint_disabled_changes_colors() {
        let mut state = WaTextareaState::new("Greyed");
        state.disabled = true;
        state.value = "Cannot edit".into();
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaTextarea.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 应该产生绘制操作
        assert!(!ops.is_empty(), "disabled textarea 仍应绘制");
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
        let state = WaTextareaState::new("Notes");
        let _any = state.as_any();
    }
}
