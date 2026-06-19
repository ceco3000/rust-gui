/// Translated from Web Awesome wa-checkbox
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

/// Web Awesome wa-checkbox 组件状态。
///
/// 跳过 name/value/title 等 Web 专属表单属性。
/// FormField trait impl 暂时跳过（WC02 trait 已存在但尚未提交到代码中）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaCheckboxState {
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 禁用状态
    pub disabled: bool,
    /// 半选状态（部分选择）
    pub indeterminate: bool,
    /// 选中状态
    pub checked: bool,
    /// 必填字段
    pub required: bool,
    /// 提示文本
    pub hint: String,
    /// 标签文本（来自默认 slot）
    pub label: String,
}

impl WaCheckboxState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: "m".into(),
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaCheckboxMessage {
    /// 失去焦点
    Blur,
    /// 选中状态改变
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

pub struct WaCheckbox;

/// 根据 size 属性返回复选框方框的像素尺寸。
fn checkbox_square_size(size: &str) -> f64 {
    match size {
        "xs" => 12.0,
        "s" | "small" => 16.0,
        "l" | "large" => 24.0,
        "xl" => 28.0,
        _ => 20.0, // "m" | "medium" (默认)
    }
}

/// 返回图标 Unicode 字符。
fn checkbox_icon_char(name: &str) -> &'static str {
    match name {
        "check" => "\u{2713}",         // ✓
        "indeterminate" => "\u{2014}", // —
        _ => "?",
    }
}

impl WidgetSpec for WaCheckbox {
    type State = WaCheckboxState;
    type Message = WaCheckboxMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaCheckbox"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaCheckbox")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("checked", PropValue::Bool(state.checked))
            .prop("indeterminate", PropValue::Bool(state.indeterminate))
            .prop("required", PropValue::Bool(state.required))
            .prop("hint", PropValue::str(state.hint.as_str()))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaCheckboxMessage::Change | WaCheckboxMessage::Input => {
                if !state.disabled {
                    state.checked = !state.checked;
                    state.indeterminate = false; // 点击后退出 semi 状态
                }
            }
            WaCheckboxMessage::Blur | WaCheckboxMessage::Focus | WaCheckboxMessage::Invalid => {}
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let box_size = checkbox_square_size(&state.size);
        let gap = 8.0; // 方框与标签之间的间距
        let font_size = box_size; // 标签字体大小与方框尺寸匹配
        let char_count = state.label.chars().count().max(1) as f64;
        let text_width = char_count * font_size * 0.6;
        let min_w = box_size + gap + text_width;
        let min_h = box_size.max(font_size * 1.2);

        Size::new(
            min_w.clamp(c.min_width, c.max_width),
            min_h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let box_size: f64 = checkbox_square_size(&state.size);
        // 将方框垂直居中于 bounds 内
        let box_y = bounds.origin.y + (bounds.size.height - box_size) / 2.0;
        let box_x = bounds.origin.x;
        let box_rect = Rect::new(box_x, box_y, box_size, box_size);

        // 方框背景
        let is_interactive = !state.disabled;
        let bg = if state.checked || state.indeterminate {
            if is_interactive {
                Color::new(0.20, 0.50, 0.90, 1.0) // 蓝色
            } else {
                Color::new(0.6, 0.6, 0.6, 1.0) // 灰色（禁用）
            }
        } else {
            Color::WHITE
        };

        let border_radius: f32 = 4.0;
        ctx.fill_rect(box_rect, bg, border_radius);

        // 方框边框（始终绘制，颜色根据状态变化）
        // 通过再画一个稍小的矩形来模拟边框效果
        let border_color = if is_interactive {
            if state.checked || state.indeterminate {
                bg
            } else {
                Color::new(0.5, 0.5, 0.5, 1.0)
            }
        } else {
            Color::new(0.7, 0.7, 0.7, 1.0)
        };
        // 填充边框色 + 内部更小的白色/蓝色区域模拟 2px 边框
        let border_inset: f64 = 2.0;
        let inner_rect = Rect::new(
            box_x + border_inset,
            box_y + border_inset,
            box_size - border_inset * 2.0,
            box_size - border_inset * 2.0,
        );
        ctx.fill_rect(box_rect, border_color, border_radius);
        ctx.fill_rect(inner_rect, bg, (border_radius as f64 - 1.0).max(1.0) as f32);

        // 绘制图标（check 或 indeterminate）
        if state.checked || state.indeterminate {
            let icon_name = if state.indeterminate { "indeterminate" } else { "check" };
            let icon = checkbox_icon_char(icon_name);
            // 将图标居中于方框内
            let icon_font_size = (box_size * 0.7) as f32;
            let icon_color = Color::WHITE;
            let icon_bounds = Rect::new(
                box_x,
                box_y,
                box_size,
                box_size,
            );
            ctx.draw_text(icon, icon_bounds, icon_color, icon_font_size);
        }

        // 绘制标签文本
        if !state.label.is_empty() {
            let gap: f64 = 8.0;
            let font_size = (box_size * 0.85) as f32;
            let text_color = if state.disabled {
                Color::new(0.6, 0.6, 0.6, 1.0)
            } else {
                Color::new(0.1, 0.1, 0.1, 1.0) // 深色文字
            };
            let text_x = box_x + box_size + gap;
            let text_bounds = Rect::new(
                text_x,
                bounds.origin.y,
                bounds.size.width - box_size - gap,
                bounds.size.height,
            );
            ctx.draw_text(&state.label, text_bounds, text_color, font_size);
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
        assert_eq!(WaCheckbox.name(), "rgui_components::WaCheckbox");
    }

    #[test]
    fn view_has_label() {
        let state = WaCheckboxState::new("Accept terms");
        let v = WaCheckbox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_checked() {
        let mut state = WaCheckboxState::new("Opt in");
        state.checked = true;
        let v = WaCheckbox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("checked"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_disabled() {
        let mut state = WaCheckboxState::new("Disabled");
        state.disabled = true;
        let v = WaCheckbox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_indeterminate() {
        let mut state = WaCheckboxState::new("Mixed");
        state.indeterminate = true;
        let v = WaCheckbox.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("indeterminate"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn update_change_toggles_checked() {
        let mut state = WaCheckboxState::new("Toggle");
        assert!(!state.checked);
        WaCheckbox.update(
            WaCheckboxMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.checked);
        // Second toggle: unchecked
        WaCheckbox.update(
            WaCheckboxMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.checked);
    }

    #[test]
    fn update_change_clears_indeterminate() {
        let mut state = WaCheckboxState::new("Semi");
        state.indeterminate = true;
        WaCheckbox.update(
            WaCheckboxMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.indeterminate);
    }

    #[test]
    fn update_disabled_does_not_toggle() {
        let mut state = WaCheckboxState::new("Locked");
        state.disabled = true;
        WaCheckbox.update(
            WaCheckboxMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.checked);
    }

    #[test]
    fn update_input_toggles() {
        let mut state = WaCheckboxState::new("Input");
        WaCheckbox.update(
            WaCheckboxMessage::Input,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.checked);
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaCheckboxState::new("OK");
        WaCheckbox.update(
            WaCheckboxMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaCheckboxState::new("Accept");
        let size = WaCheckbox.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 40.0, "宽度应 ≥ 40px，实际 {size:?}");
        assert!(size.height >= 20.0, "高度应 ≥ 20px，实际 {size:?}");
    }

    #[test]
    fn measure_size_xs_smaller() {
        let mut state = WaCheckboxState::new("Small");
        state.size = "xs".into();
        let xs = WaCheckbox.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        state.size = "xl".into();
        let xl = WaCheckbox.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(xs.width < xl.width, "xs 应比 xl 窄");
    }

    #[test]
    fn paint_unchecked_produces_ops() {
        let state = WaCheckboxState::new("Unchecked");
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaCheckbox.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "未选中复选框应至少绘制边框+内填充+标签");
    }

    #[test]
    fn paint_checked_produces_ops() {
        let mut state = WaCheckboxState::new("Checked");
        state.checked = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaCheckbox.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "选中复选框应绘制边框+填充+图标+标签");
    }

    #[test]
    fn paint_disabled_shows_gray() {
        let mut state = WaCheckboxState::new("Disabled");
        state.disabled = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaCheckbox.paint(&state, bounds, &mut ctx);
        // 禁用状态仍应产生绘制操作
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaCheckboxMessage::Change.message_name(), "change");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaCheckboxState::schema_name(), "WaCheckboxState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaCheckboxState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaCheckboxState>());
    }
}
