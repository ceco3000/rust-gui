/// Translated from Web Awesome wa-radio
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

/// Web Awesome wa-radio 组件状态。
///
/// Radio 是单选按钮，表示互斥选项集中的单个选项。
/// FormField trait impl 暂时跳过（WC02 trait 存在但尚未提交到代码中）。
/// 跳过 forceDisabled（RadioGroup 内部状态，由容器管理）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaRadioState {
    /// 选中状态
    pub checked: bool,
    /// Radio 的值，选中时提交给 RadioGroup
    pub value: String,
    /// 视觉外观：default（圆形）| button（矩形按钮）
    pub appearance: String,
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 禁用状态
    pub disabled: bool,
    /// 标签文本（来自默认 slot）
    pub label: String,
}

impl WaRadioState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: "m".into(),
            appearance: "default".into(),
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaRadioMessage {
    /// 失去焦点
    Blur,
    /// 获得焦点
    Focus,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaRadio;

/// 根据 size 属性返回 Radio 圆圈直径（像素）。
fn radio_square_size(size: &str) -> f64 {
    match size {
        "xs" => 12.0,
        "s" | "small" => 16.0,
        "l" | "large" => 24.0,
        "xl" => 28.0,
        _ => 20.0, // "m" | "medium" (默认)
    }
}

impl WidgetSpec for WaRadio {
    type State = WaRadioState;
    type Message = WaRadioMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaRadio"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaRadio")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("checked", PropValue::Bool(state.checked))
            .prop("value", PropValue::str(state.value.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaRadioMessage::Blur | WaRadioMessage::Focus => {}
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let box_size = radio_square_size(&state.size);
        let gap = 8.0; // 圆圈与标签之间的间距
        let font_size = box_size; // 标签字体大小与圆圈尺寸匹配
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
        let box_size: f64 = radio_square_size(&state.size);
        // 将圆圈垂直居中于 bounds 内
        let circle_y = bounds.origin.y + (bounds.size.height - box_size) / 2.0;
        let circle_x = bounds.origin.x;

        let is_interactive = !state.disabled;
        let is_button = state.appearance == "button";

        if is_button {
            // ── 按钮外观：矩形背景 ──
            let bg = if state.checked {
                if is_interactive {
                    Color::new(0.20, 0.50, 0.90, 1.0)
                } else {
                    Color::new(0.6, 0.6, 0.6, 1.0)
                }
            } else {
                Color::new(0.95, 0.95, 0.95, 1.0)
            };

            let border_color = if is_interactive {
                if state.checked {
                    bg
                } else {
                    Color::new(0.5, 0.5, 0.5, 1.0)
                }
            } else {
                Color::new(0.7, 0.7, 0.7, 1.0)
            };

            let border_radius: f32 = 4.0;
            let btn_rect = Rect::new(circle_x, circle_y, box_size + 16.0, box_size + 8.0);
            ctx.fill_rect(btn_rect, border_color, border_radius);

            let border_inset: f64 = 1.0;
            let inner = Rect::new(
                btn_rect.origin.x + border_inset,
                btn_rect.origin.y + border_inset,
                btn_rect.size.width - border_inset * 2.0,
                btn_rect.size.height - border_inset * 2.0,
            );
            ctx.fill_rect(inner, bg, border_radius);

            // 标签文本
            if !state.label.is_empty() {
                let gap: f64 = 8.0;
                let font_size = (box_size * 0.85) as f32;
                let text_color = if state.disabled {
                    Color::new(0.6, 0.6, 0.6, 1.0)
                } else {
                    Color::new(0.1, 0.1, 0.1, 1.0)
                };
                let text_x = btn_rect.origin.x + btn_rect.size.width + gap;
                let text_bounds = Rect::new(
                    text_x,
                    bounds.origin.y,
                    bounds.size.width - btn_rect.size.width - gap,
                    bounds.size.height,
                );
                ctx.draw_text(&state.label, text_bounds, text_color, font_size);
            }
        } else {
            // ── 默认外观：圆形 ──
            // 绘制外圈（使用 fill_rect + 最大圆角模拟圆形，或使用多层矩形逼近）
            let outer_color = if is_interactive {
                if state.checked {
                    Color::new(0.20, 0.50, 0.90, 1.0) // 蓝色外圈
                } else {
                    Color::new(0.5, 0.5, 0.5, 1.0) // 灰色外圈
                }
            } else {
                Color::new(0.7, 0.7, 0.7, 1.0) // 禁用灰色
            };

            let outer_rect = Rect::new(circle_x, circle_y, box_size, box_size);
            let outer_r: f32 = (box_size / 2.0) as f32;
            ctx.fill_rect(outer_rect, outer_color, outer_r);

            // 内部白色区域（模拟空心圆）
            let ring_thickness: f64 = 2.0;
            let inner_rect = Rect::new(
                circle_x + ring_thickness,
                circle_y + ring_thickness,
                box_size - ring_thickness * 2.0,
                box_size - ring_thickness * 2.0,
            );
            let inner_r: f32 = ((box_size - ring_thickness * 2.0) / 2.0) as f32;
            ctx.fill_rect(inner_rect, Color::WHITE, inner_r);

            // 选中时绘制内部填充圆点
            if state.checked {
                let dot_inset: f64 = box_size * 0.3;
                let dot_rect = Rect::new(
                    circle_x + dot_inset,
                    circle_y + dot_inset,
                    box_size - dot_inset * 2.0,
                    box_size - dot_inset * 2.0,
                );
                let dot_r: f32 = ((box_size - dot_inset * 2.0) / 2.0) as f32;
                let dot_color = if is_interactive {
                    Color::new(0.20, 0.50, 0.90, 1.0)
                } else {
                    Color::new(0.6, 0.6, 0.6, 1.0)
                };
                ctx.fill_rect(dot_rect, dot_color, dot_r);
            }

            // 绘制标签文本
            if !state.label.is_empty() {
                let gap: f64 = 8.0;
                let font_size = (box_size * 0.85) as f32;
                let text_color = if state.disabled {
                    Color::new(0.6, 0.6, 0.6, 1.0)
                } else {
                    Color::new(0.1, 0.1, 0.1, 1.0)
                };
                let text_x = circle_x + box_size + gap;
                let text_bounds = Rect::new(
                    text_x,
                    bounds.origin.y,
                    bounds.size.width - box_size - gap,
                    bounds.size.height,
                );
                ctx.draw_text(&state.label, text_bounds, text_color, font_size);
            }
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
        assert_eq!(WaRadio.name(), "rgui_components::WaRadio");
    }

    #[test]
    fn view_has_label() {
        let state = WaRadioState::new("Option A");
        let v = WaRadio.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_checked() {
        let mut state = WaRadioState::new("Selected");
        state.checked = true;
        let v = WaRadio.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("checked"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_disabled() {
        let mut state = WaRadioState::new("Disabled");
        state.disabled = true;
        let v = WaRadio.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_has_value() {
        let mut state = WaRadioState::new("Value");
        state.value = "radio-1".into();
        let v = WaRadio.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("value"), Some(&PropValue::Str("radio-1".into())));
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaRadioState::new("OK");
        WaRadio.update(
            WaRadioMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_focus_is_handled() {
        let mut state = WaRadioState::new("OK");
        WaRadio.update(
            WaRadioMessage::Focus,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaRadioState::new("Option");
        let size = WaRadio.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 30.0, "宽度应 ≥ 30px，实际 {size:?}");
        assert!(size.height >= 20.0, "高度应 ≥ 20px，实际 {size:?}");
    }

    #[test]
    fn measure_size_xs_smaller() {
        let mut state = WaRadioState::new("Small");
        state.size = "xs".into();
        let xs = WaRadio.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        state.size = "xl".into();
        let xl = WaRadio.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(xs.width < xl.width, "xs 应比 xl 窄");
    }

    #[test]
    fn paint_unchecked_produces_ops() {
        let state = WaRadioState::new("Unchecked");
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaRadio.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "未选中 Radio 应至少绘制外圈+内白+标签");
    }

    #[test]
    fn paint_checked_produces_ops() {
        let mut state = WaRadioState::new("Checked");
        state.checked = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaRadio.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "选中 Radio 应绘制外圈+内白+圆点+标签");
    }

    #[test]
    fn paint_disabled_shows_gray() {
        let mut state = WaRadioState::new("Disabled");
        state.disabled = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaRadio.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_empty_label_no_crash() {
        let state = WaRadioState::new("");
        let bounds = Rect::new(0.0, 0.0, 40.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        WaRadio.paint(&state, bounds, &mut ctx);
        // 无标签 → 只有圆圈绘制操作
        assert!(ctx.op_count() >= 2);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaRadioMessage::Blur.message_name(), "blur");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaRadioState::schema_name(), "WaRadioState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaRadioState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaRadioState>());
    }
}
