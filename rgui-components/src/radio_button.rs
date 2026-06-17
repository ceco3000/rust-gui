//! RadioButton 组件——单选按钮。
//!
//! 支持分组选择（同一组内只能选中一项）、禁用状态。
//! 发送 [`RadioButtonMessage`] 消息。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};

/// RadioButton 业务状态。
///
/// 包含标签文本、是否选中、禁用标志和分组名称。
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct RadioButtonState {
    /// 单选按钮标签文本。
    pub label: String,
    /// 是否处于选中状态。
    pub selected: bool,
    /// 是否禁用。
    pub disabled: bool,
    /// 所属分组名称。同一组内只能有一个 RadioButton 被选中。
    pub group: String,
}
impl RadioButtonState {
    /// 创建新的 RadioButtonState，指定标签和分组。
    #[must_use]
    pub fn new(label: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            group: group.into(),
            ..Self::default()
        }
    }
}

/// RadioButton 消息类型。
///
/// - `Select`: 选中此选项
/// - `FocusIn` / `FocusOut`: 焦点变化
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum RadioButtonMessage {
    Select,
    FocusIn,
    FocusOut,
}

/// RadioButton 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。用于单选场景。
pub struct RadioButton;
impl WidgetSpec for RadioButton {
    type State = RadioButtonState;
    type Message = RadioButtonMessage;
    fn name(&self) -> &'static str {
        "rgui_components::RadioButton"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("RadioButton")
            .prop("label", PropValue::str(s.label.as_str()))
            .prop("selected", PropValue::Bool(s.selected))
            .prop("group", PropValue::str(s.group.as_str()))
    }
    fn update(&self, msg: Self::Message, s: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            RadioButtonMessage::Select if !s.disabled => s.selected = true,
            _ => {},
        }
    }
    fn measure(&self, _: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(
            150_f64.clamp(c.min_width, c.max_width),
            24_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let radius = 9.0_f64.min(bounds.size.height * 0.5);
        let cx = bounds.origin.x + radius + 4.0;
        let cy = bounds.origin.y + bounds.size.height * 0.5;
        // 外圈
        let ring_rect = Rect::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
        let ring_color = if s.disabled {
            Color::new(0.4, 0.4, 0.4, 1.0)
        } else {
            Color::new(0.7, 0.7, 0.8, 1.0)
        };
        ctx.fill_rect(ring_rect, ring_color, radius as f32);

        // 内圈（选中时）
        if s.selected && !s.disabled {
            let inner_r = radius * 0.5;
            ctx.fill_rect(
                Rect::new(cx - inner_r, cy - inner_r, inner_r * 2.0, inner_r * 2.0),
                Color::new(0.20, 0.55, 0.95, 1.0),
                inner_r as f32,
            );
        }

        // 标签文本（对齐 Button：间距 4px + 字号按高度比例）
        let text_x = cx + radius + 4.0;
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
        let font_size = bounds.size.height as f32 * 0.8;
        ctx.draw_text(&s.label, text_bounds, text_color, font_size);
    }
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(s.label.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn select() {
        let mut s = RadioButtonState::new("A", "g1");
        RadioButton.update(
            RadioButtonMessage::Select,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.selected);
    }
    #[test]
    fn disabled() {
        let mut s = RadioButtonState::new("B", "g1");
        s.disabled = true;
        RadioButton.update(
            RadioButtonMessage::Select,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.selected);
    }
    #[test]
    fn group() {
        assert_eq!(RadioButtonState::new("A", "theme").group, "theme");
    }

    #[test]
    fn paint_selected() {
        let state = RadioButtonState::new("Option A", "group1");
        let mut s = state;
        s.selected = true;
        let bounds = Rect::new(0.0, 0.0, 150.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        RadioButton.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "选中应绘制外圈 + 内圆 + 文本");
    }

    #[test]
    fn paint_not_selected() {
        let state = RadioButtonState::new("Option B", "group1");
        let bounds = Rect::new(0.0, 0.0, 150.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        RadioButton.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2);
    }
}
