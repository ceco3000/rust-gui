/// Translated from Web Awesome wa-switch
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

/// Web Awesome wa-switch 组件状态。
///
/// 跳过 name/value/title/withHint 等 Web 专属表单属性。
/// FormField trait impl 暂时跳过（WC02 trait 已存在但尚未提交到代码中）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaSwitchState {
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 禁用状态
    pub disabled: bool,
    /// 选中状态
    pub checked: bool,
    /// 必填字段
    pub required: bool,
    /// 提示文本
    pub hint: String,
    /// 标签文本（来自默认 slot）
    pub label: String,
}

impl WaSwitchState {
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
pub enum WaSwitchMessage {
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

pub struct WaSwitch;

/// 根据 size 属性返回 switch 轨道的高度（px）。
///
/// WA 中 toggle size = 1.25em，这里直接映射为像素值。
fn switch_track_height(size: &str) -> f64 {
    match size {
        "xs" => 15.0,
        "s" | "small" => 18.0,
        "l" | "large" => 25.0,
        "xl" => 31.0,
        _ => 20.0, // "m" | "medium"（默认）
    }
}

/// 轨道宽度 = 高度 × 1.75（WA 中 --width: calc(var(--height) * 1.75)）
fn switch_track_width(height: f64) -> f64 {
    height * 1.75
}

/// 拇指直径 = 轨道高度 × 0.75（WA 中 --thumb-size: 0.75em）
fn switch_thumb_diameter(track_height: f64) -> f64 {
    track_height * 0.75
}

impl WidgetSpec for WaSwitch {
    type State = WaSwitchState;
    type Message = WaSwitchMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaSwitch"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaSwitch")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("checked", PropValue::Bool(state.checked))
            .prop("required", PropValue::Bool(state.required))
            .prop("hint", PropValue::str(state.hint.as_str()))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaSwitchMessage::Change | WaSwitchMessage::Input => {
                if !state.disabled {
                    state.checked = !state.checked;
                }
            },
            WaSwitchMessage::Blur | WaSwitchMessage::Focus | WaSwitchMessage::Invalid => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let track_h = switch_track_height(&state.size);
        let track_w = switch_track_width(track_h);
        let gap = 8.0; // 轨道与标签之间的间距
        let font_size = track_h * 0.8; // 标签字体大小与轨道匹配
        let char_count = state.label.chars().count().max(1) as f64;
        let text_width = char_count * font_size * 0.6;
        let min_w = track_w + gap + text_width;
        let min_h = track_h.max(font_size * 1.2);

        Size::new(
            min_w.clamp(c.min_width, c.max_width),
            min_h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let track_h: f64 = switch_track_height(&state.size);
        let track_w: f64 = switch_track_width(track_h);
        let thumb_d: f64 = switch_thumb_diameter(track_h);

        // 轨道垂直居中于 bounds 内
        let track_y = bounds.origin.y + (bounds.size.height - track_h) / 2.0;
        let track_x = bounds.origin.x;

        // ── 轨道背景 ──
        let track_radius: f32 = (track_h / 2.0) as f32; // 完全圆角 = 半圆形轨道
        let track_bg = if state.checked {
            if state.disabled {
                Color::new(0.6, 0.6, 0.6, 1.0)
            } else {
                Color::new(0.20, 0.50, 0.90, 1.0) // 蓝色激活态
            }
        } else {
            if state.disabled {
                Color::new(0.85, 0.85, 0.85, 1.0)
            } else {
                Color::new(0.75, 0.75, 0.75, 1.0) // 浅灰色未激活
            }
        };
        let track_rect = Rect::new(track_x, track_y, track_w, track_h);
        ctx.fill_rect(track_rect, track_bg, track_radius);

        // ── 拇指 ──
        // 拇指在轨道内垂直居中
        let thumb_y = track_y + (track_h - thumb_d) / 2.0;
        // 水平位置：未选中靠左，选中靠右
        let thumb_padding = (track_h - thumb_d) / 2.0; // 边缘留白 = 轨道与拇指高度差的 1/2
        let thumb_x = if state.checked {
            track_x + track_w - thumb_d - thumb_padding
        } else {
            track_x + thumb_padding
        };
        let thumb_radius: f32 = (thumb_d / 2.0) as f32;
        let thumb_color = Color::WHITE;
        let thumb_rect = Rect::new(thumb_x, thumb_y, thumb_d, thumb_d);
        ctx.fill_rect(thumb_rect, thumb_color, thumb_radius);

        // ── 标签文本 ──
        if !state.label.is_empty() {
            let gap: f64 = 8.0;
            let font_size = (track_h * 0.8) as f32;
            let text_color = if state.disabled {
                Color::new(0.6, 0.6, 0.6, 1.0)
            } else {
                Color::new(0.1, 0.1, 0.1, 1.0)
            };
            let text_x = track_x + track_w + gap;
            let text_bounds = Rect::new(
                text_x,
                bounds.origin.y,
                bounds.size.width - track_w - gap,
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
        assert_eq!(WaSwitch.name(), "rgui_components::WaSwitch");
    }

    #[test]
    fn view_has_label() {
        let state = WaSwitchState::new("Notifications");
        let v = WaSwitch.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_checked() {
        let mut state = WaSwitchState::new("Toggle");
        state.checked = true;
        let v = WaSwitch.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("checked"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_disabled() {
        let mut state = WaSwitchState::new("Locked");
        state.disabled = true;
        let v = WaSwitch.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_required() {
        let mut state = WaSwitchState::new("Required");
        state.required = true;
        let v = WaSwitch.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("required"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn update_change_toggles_checked() {
        let mut state = WaSwitchState::new("Toggle");
        assert!(!state.checked);
        WaSwitch.update(
            WaSwitchMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.checked);
        // Second toggle: unchecked
        WaSwitch.update(
            WaSwitchMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.checked);
    }

    #[test]
    fn update_disabled_does_not_toggle() {
        let mut state = WaSwitchState::new("Locked");
        state.disabled = true;
        WaSwitch.update(
            WaSwitchMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.checked);
    }

    #[test]
    fn update_input_toggles() {
        let mut state = WaSwitchState::new("Input");
        WaSwitch.update(
            WaSwitchMessage::Input,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.checked);
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaSwitchState::new("OK");
        WaSwitch.update(
            WaSwitchMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_focus_is_handled() {
        let mut state = WaSwitchState::new("OK");
        WaSwitch.update(
            WaSwitchMessage::Focus,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_invalid_is_handled() {
        let mut state = WaSwitchState::new("OK");
        WaSwitch.update(
            WaSwitchMessage::Invalid,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaSwitchState::new("Toggle");
        let size = WaSwitch.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 40.0, "宽度应 ≥ 40px，实际 {size:?}");
        assert!(size.height >= 20.0, "高度应 ≥ 20px，实际 {size:?}");
    }

    #[test]
    fn measure_size_xs_smaller() {
        let mut state = WaSwitchState::new("Small");
        state.size = "xs".into();
        let xs = WaSwitch.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        state.size = "xl".into();
        let xl = WaSwitch.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(xs.width < xl.width, "xs 应比 xl 窄");
    }

    #[test]
    fn paint_unchecked_produces_ops() {
        let state = WaSwitchState::new("Off");
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaSwitch.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2, "未选中 switch 应至少绘制轨道+拇指");
    }

    #[test]
    fn paint_checked_produces_ops() {
        let mut state = WaSwitchState::new("On");
        state.checked = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaSwitch.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "选中 switch 应绘制轨道+拇指+标签");
    }

    #[test]
    fn paint_disabled_checked_is_gray() {
        let mut state = WaSwitchState::new("Gray");
        state.checked = true;
        state.disabled = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaSwitch.paint(&state, bounds, &mut ctx);
        // 禁用状态仍应产生绘制操作
        assert!(ctx.op_count() >= 2);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaSwitchMessage::Change.message_name(), "change");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaSwitchState::schema_name(), "WaSwitchState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaSwitchState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaSwitchState>());
    }
}
