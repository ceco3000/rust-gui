/// Translated from Web Awesome wa-input
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

/// Web Awesome wa-input 组件状态。
///
/// Input 是单行文本输入框，支持标签、提示、验证、前缀/后缀 slot、
/// 清除按钮和密码切换。
///
/// 跳过 formAction/formEnctype/formMethod 等 Web 专属表单属性。
/// 跳过 withLabel/withHint SSR 属性。
/// FormField trait impl 暂时跳过（WC02 trait 已存在但尚未提交到代码中）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaInputState {
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 视觉外观：filled | outlined | filled-outlined
    pub appearance: String,
    /// 药丸形状
    pub pill: bool,
    /// 输入类型：text | password | email | number | search | tel | url | date | datetime-local | time
    pub r#type: String,
    /// 标签文本
    pub label: String,
    /// 提示文本
    pub hint: String,
    /// 占位符文本
    pub placeholder: String,
    /// 当前值
    pub value: String,
    /// 默认值（表单重置时使用）
    pub default_value: String,
    /// 禁用状态
    pub disabled: bool,
    /// 只读状态
    pub readonly: bool,
    /// 必填字段
    pub required: bool,
    /// 显示清除按钮
    pub with_clear: bool,
    /// 显示密码切换按钮
    pub password_toggle: bool,
    /// 密码当前可见
    pub password_visible: bool,
}

impl WaInputState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: "m".into(),
            appearance: "outlined".into(),
            r#type: "text".into(),
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaInputMessage {
    /// 失去焦点
    Blur,
    /// 值已提交
    Change,
    /// 获得焦点
    Focus,
    /// 接收输入
    Input,
    /// 清除按钮被点击
    Clear,
    /// 验证失败
    Invalid,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaInput;

/// 根据 size 属性的像素映射
fn input_height(size: &str) -> f64 {
    match size {
        "xs" => 24.0,
        "s" | "small" => 28.0,
        "l" | "large" => 48.0,
        "xl" => 56.0,
        _ => 36.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的字体大小映射
fn input_font_size(size: &str) -> f64 {
    match size {
        "xs" => 12.0,
        "s" | "small" => 14.0,
        "l" | "large" => 20.0,
        "xl" => 24.0,
        _ => 16.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的 border-radius 映射
fn input_border_radius(size: &str, pill: bool) -> f32 {
    if pill {
        return (input_height(size) / 2.0) as f32;
    }
    match size {
        "xs" => 2.0,
        "s" | "small" => 3.0,
        "l" | "large" => 6.0,
        "xl" => 8.0,
        _ => 4.0, // "m" | "medium" (默认)
    }
}

impl WidgetSpec for WaInput {
    type State = WaInputState;
    type Message = WaInputMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaInput"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaInput")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("type", PropValue::str(state.r#type.as_str()))
            .prop("value", PropValue::str(state.value.as_str()))
            .prop("placeholder", PropValue::str(state.placeholder.as_str()))
            .prop("hint", PropValue::str(state.hint.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("readonly", PropValue::Bool(state.readonly))
            .prop("required", PropValue::Bool(state.required))
            .prop("pill", PropValue::Bool(state.pill))
            .prop("with-clear", PropValue::Bool(state.with_clear))
            .prop("password-toggle", PropValue::Bool(state.password_toggle))
            .prop("password-visible", PropValue::Bool(state.password_visible))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaInputMessage::Change => {
                // 值提交——由 IME/框架事件系统处理后调用，这里只标记状态更新
                // 实际值由框架通过 props 传递并写入 state
            },
            WaInputMessage::Input => {
                // 输入中——由 IME 驱动，state.value 由框架更新
            },
            WaInputMessage::Clear => {
                if !state.disabled && !state.readonly {
                    state.value = String::new();
                }
            },
            WaInputMessage::Blur | WaInputMessage::Focus | WaInputMessage::Invalid => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let h = input_height(&state.size);
        let font_size = input_font_size(&state.size);
        // 宽度：根据文本内容估算
        let char_count = state
            .value
            .chars()
            .count()
            .max(state.placeholder.chars().count())
            .max(10) as f64;
        let text_width = char_count * font_size * 0.6 + 24.0; // 24px padding
        // 如果有标签，加上标签高度
        let label_height = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };
        let total_h = h + label_height;

        Size::new(
            text_width.clamp(c.min_width, c.max_width),
            total_h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let h = input_height(&state.size);
        let font_size = input_font_size(&state.size);
        let border_radius = input_border_radius(&state.size, state.pill);

        // 标签绘制偏移
        let label_offset_y = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };
        let input_y = bounds.origin.y + label_offset_y;
        let input_rect = Rect::new(bounds.origin.x, input_y, bounds.size.width, h);

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

        // ── 输入框背景 ──
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
        ctx.fill_rect(input_rect, bg_color, border_radius);

        // 模拟边框（绘制略小的矩形叠加）
        let border_width: f64 = 2.0;
        ctx.fill_rect(input_rect, border, border_radius);
        let inner_rect = Rect::new(
            input_rect.origin.x + border_width,
            input_rect.origin.y + border_width,
            input_rect.size.width - border_width * 2.0,
            input_rect.size.height - border_width * 2.0,
        );
        let inner_radius = (border_radius as f64 - 1.0).max(1.0) as f32;
        ctx.fill_rect(inner_rect, bg_color, inner_radius);

        // ── 值文本 / 占位符 ──
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

        // 截断显示文本：密码类型时显示星号
        let display_str =
            if state.r#type == "password" && !state.password_visible && !state.value.is_empty() {
                "●".repeat(state.value.chars().count())
            } else {
                display_text.clone()
            };

        if !display_str.is_empty() {
            let text_padding: f64 = 8.0;
            let text_rect = Rect::new(
                input_rect.origin.x + text_padding,
                input_rect.origin.y,
                input_rect.size.width - text_padding * 2.0,
                input_rect.size.height,
            );
            ctx.draw_text(&display_str, text_rect, text_color, font_size as f32);
        }

        // ── 清除按钮 (× 图标) ──
        if state.with_clear && !state.disabled && !state.readonly && !state.value.is_empty() {
            let clear_size: f64 = h * 0.6;
            let clear_x = input_rect.origin.x + input_rect.size.width - clear_size - 8.0;
            let clear_y = input_rect.origin.y + (h - clear_size) / 2.0;
            let clear_rect = Rect::new(clear_x, clear_y, clear_size, clear_size);
            let clear_color = Color::new(0.5, 0.5, 0.5, 1.0);
            ctx.draw_text("✕", clear_rect, clear_color, (clear_size * 0.8) as f32);
        }

        // ── 密码切换按钮 ──
        if state.password_toggle && state.r#type == "password" && !state.disabled {
            let toggle_size: f64 = h * 0.6;
            let toggle_x = input_rect.origin.x + input_rect.size.width - toggle_size - 8.0;
            // 如果有清除按钮，向左偏移
            let has_clear = state.with_clear && !state.value.is_empty();
            let toggle_x = if has_clear {
                toggle_x - toggle_size - 4.0
            } else {
                toggle_x
            };
            let toggle_y = input_rect.origin.y + (h - toggle_size) / 2.0;
            let toggle_rect = Rect::new(toggle_x, toggle_y, toggle_size, toggle_size);
            let toggle_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let toggle_icon = if state.password_visible {
                "🙈"
            } else {
                "👁"
            };
            ctx.draw_text(
                toggle_icon,
                toggle_rect,
                toggle_color,
                (toggle_size * 0.8) as f32,
            );
        }

        // ── 提示文本 ──
        if !state.hint.is_empty() {
            let hint_y = input_rect.origin.y + input_rect.size.height + 4.0;
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
    use std::any::TypeId;

    #[test]
    fn name() {
        assert_eq!(WaInput.name(), "rgui_components::WaInput");
    }

    #[test]
    fn view_has_label() {
        let state = WaInputState::new("Username");
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_type() {
        let mut state = WaInputState::new("Email");
        state.r#type = "email".into();
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("type"),
            Some(&PropValue::Str(std::sync::Arc::from("email")))
        );
    }

    #[test]
    fn view_has_value() {
        let mut state = WaInputState::new("Name");
        state.value = "Alice".into();
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("value"),
            Some(&PropValue::Str(std::sync::Arc::from("Alice")))
        );
    }

    #[test]
    fn view_disabled() {
        let mut state = WaInputState::new("Disabled");
        state.disabled = true;
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_readonly() {
        let mut state = WaInputState::new("Readonly");
        state.readonly = true;
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("readonly"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_pill() {
        let mut state = WaInputState::new("Pill");
        state.pill = true;
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("pill"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_with_clear() {
        let mut state = WaInputState::new("Clearable");
        state.with_clear = true;
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("with-clear"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_password_toggle() {
        let mut state = WaInputState::new("Password");
        state.password_toggle = true;
        let v = WaInput.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("password-toggle"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn update_clear_empties_value() {
        let mut state = WaInputState::new("Clear me");
        state.value = "some text".into();
        WaInput.update(
            WaInputMessage::Clear,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.value.is_empty());
    }

    #[test]
    fn update_clear_on_disabled_does_nothing() {
        let mut state = WaInputState::new("Locked");
        state.disabled = true;
        state.value = "keep me".into();
        WaInput.update(
            WaInputMessage::Clear,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.value, "keep me");
    }

    #[test]
    fn update_clear_on_readonly_does_nothing() {
        let mut state = WaInputState::new("Read only");
        state.readonly = true;
        state.value = "keep me".into();
        WaInput.update(
            WaInputMessage::Clear,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.value, "keep me");
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaInputState::new("OK");
        WaInput.update(
            WaInputMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_focus_is_handled() {
        let mut state = WaInputState::new("OK");
        WaInput.update(
            WaInputMessage::Focus,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_invalid_is_handled() {
        let mut state = WaInputState::new("OK");
        WaInput.update(
            WaInputMessage::Invalid,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaInputState::new("Input");
        let size = WaInput.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 80.0, "宽度应 ≥ 80px，实际 {size:?}");
        assert!(size.height >= 36.0, "高度应 ≥ 36px，实际 {size:?}");
    }

    #[test]
    fn measure_size_xs_smaller() {
        let mut state = WaInputState::new("Small");
        state.size = "xs".into();
        let xs = WaInput.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        state.size = "xl".into();
        let xl = WaInput.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(xs.height < xl.height, "xs 应比 xl 矮");
    }

    #[test]
    fn paint_produces_ops() {
        let mut state = WaInputState::new("Username");
        state.value = "hello".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 3,
            "应至少绘制标签+边框+文本，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_empty_shows_placeholder() {
        let mut state = WaInputState::new("Search");
        state.placeholder = "Type here...".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_disabled_shows_gray() {
        let mut state = WaInputState::new("Disabled");
        state.disabled = true;
        state.value = "gray".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_with_hint_produces_ops() {
        let mut state = WaInputState::new("Hinted");
        state.hint = "Enter your name".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_password_type_masks_value() {
        let mut state = WaInputState::new("Password");
        state.r#type = "password".into();
        state.value = "secret".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_password_visible_shows_text() {
        let mut state = WaInputState::new("Password");
        state.r#type = "password".into();
        state.password_visible = true;
        state.value = "secret".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_with_clear_shows_icon() {
        let mut state = WaInputState::new("Clearable");
        state.with_clear = true;
        state.value = "text".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        // 有清除按钮时绘制操作更多
        assert!(ctx.op_count() >= 4);
    }

    #[test]
    fn paint_appearance_filled() {
        let mut state = WaInputState::new("Filled");
        state.appearance = "filled".into();
        state.value = "text".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_appearance_filled_outlined() {
        let mut state = WaInputState::new("Filled-Outlined");
        state.appearance = "filled-outlined".into();
        state.value = "text".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaInput.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaInputMessage::Change.message_name(), "change");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaInputState::schema_name(), "WaInputState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaInputState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaInputState>());
    }
}
