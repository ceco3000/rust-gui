//! Switch 组件——开关切换。
//!
//! 支持开/关两种状态的切换、禁用状态。
//! 发送 `SwitchMessage` 消息。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};

/// Switch 业务状态。
///
/// 包含开关状态、禁用标志和标签文本。
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct SwitchState {
    /// 是否处于打开状态。
    pub on: bool,
    /// 是否禁用。
    pub disabled: bool,
    /// 开关标签文本。
    pub label: String,
}
impl SwitchState {
    /// 创建新的 SwitchState，指定标签文本。
    #[must_use]
    pub fn new(l: impl Into<String>) -> Self {
        Self {
            label: l.into(),
            ..Self::default()
        }
    }
}

/// Switch 消息类型。
///
/// - `Toggle(bool)`: 切换到指定状态
/// - `FocusIn` / `FocusOut`: 焦点变化
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum SwitchMessage {
    Toggle(bool),
    FocusIn,
    FocusOut,
}

/// Switch 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。用于二元开关场景。
pub struct Switch;
impl WidgetSpec for Switch {
    type State = SwitchState;
    type Message = SwitchMessage;
    fn name(&self) -> &'static str {
        "rgui_components::Switch"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("Switch")
            .prop("on", PropValue::Bool(s.on))
            .prop("label", PropValue::str(s.label.as_str()))
    }
    fn update(&self, msg: Self::Message, s: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            SwitchMessage::Toggle(v) if !s.disabled => s.on = v,
            _ => {},
        }
    }
    fn measure(&self, _: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(
            60_f64.clamp(c.min_width, c.max_width),
            28_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let track_height = 20.0_f64.min(bounds.size.height);
        let track_width = 40.0_f64.min(bounds.size.width * 0.5);
        let track_y = bounds.origin.y + (bounds.size.height - track_height) * 0.5;
        let track_x = bounds.origin.x + 4.0;

        // 轨道背景
        let track_color = if s.disabled {
            Color::new(0.35, 0.35, 0.35, 1.0)
        } else if s.on {
            Color::new(0.20, 0.60, 0.40, 1.0)
        } else {
            Color::new(0.4, 0.4, 0.45, 1.0)
        };
        ctx.fill_rect(
            Rect::new(track_x, track_y, track_width, track_height),
            track_color,
            track_height as f32 * 0.5,
        );

        // 滑块（圆形）
        let knob_r = track_height * 0.4;
        let knob_cx = if s.on {
            track_x + track_width - knob_r - 2.0
        } else {
            track_x + knob_r + 2.0
        };
        let knob_rect = Rect::new(
            knob_cx - knob_r,
            track_y + track_height * 0.5 - knob_r,
            knob_r * 2.0,
            knob_r * 2.0,
        );
        let knob_color = if s.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            Color::WHITE
        };
        ctx.fill_rect(knob_rect, knob_color, knob_r as f32);

        // 标签文本
        let text_x = track_x + track_width + 8.0;
        let text_bounds = Rect::new(
            text_x,
            bounds.origin.y,
            bounds.size.width - (text_x - bounds.origin.x) - 4.0,
            bounds.size.height,
        );
        let text_color = if s.disabled {
            Color::new(0.5, 0.5, 0.5, 1.0)
        } else {
            Color::new(0.9, 0.9, 0.95, 1.0)
        };
        ctx.draw_text(&s.label, text_bounds, text_color, 14.0);
    }
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(s.label.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle() {
        let mut s = SwitchState::new("");
        Switch.update(
            SwitchMessage::Toggle(true),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.on);
    }
    #[test]
    fn disabled() {
        let mut s = SwitchState::new("");
        s.disabled = true;
        Switch.update(
            SwitchMessage::Toggle(true),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.on);
    }

    #[test]
    fn paint_on_state() {
        let mut s = SwitchState::new("WiFi");
        s.on = true;
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        Switch.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "应绘制轨道 + 滑块 + 文本");
    }

    #[test]
    fn paint_off_state() {
        let s = SwitchState::new("WiFi");
        let bounds = Rect::new(0.0, 0.0, 120.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        Switch.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }
}
