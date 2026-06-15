//! CheckBox 组件——复选框。
//!
//! 支持选中/未选中状态切换、禁用状态。
//! 发送 `CheckBoxMessage` 消息。

use rgui_core::a11y::{AccessibilityAction, AccessibilityNode};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};

/// CheckBox 业务状态。
///
/// 包含标签文本、选中状态和禁用标志。
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct CheckBoxState {
    /// 复选框标签文本。
    pub label: String,
    /// 是否处于选中状态。
    pub checked: bool,
    /// 是否禁用。
    pub disabled: bool,
}
impl CheckBoxState {
    /// 创建新的 CheckBoxState，指定标签文本。
    #[must_use]
    pub fn new(l: impl Into<String>) -> Self {
        Self {
            label: l.into(),
            ..Self::default()
        }
    }
    /// 设置选中状态（builder 风格）。
    #[must_use]
    pub fn checked(mut self, v: bool) -> Self {
        self.checked = v;
        self
    }
}

/// CheckBox 消息类型。
///
/// - `Toggle(bool)`: 切换到指定选中状态
/// - `FocusIn` / `FocusOut`: 焦点变化
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum CheckBoxMessage {
    Toggle(bool),
    FocusIn,
    FocusOut,
}

/// CheckBox 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。支持选中状态切换。
pub struct CheckBox;
impl WidgetSpec for CheckBox {
    type State = CheckBoxState;
    type Message = CheckBoxMessage;
    fn name(&self) -> &'static str {
        "rgui_components::CheckBox"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("CheckBox")
            .prop("label", PropValue::str(s.label.as_str()))
            .prop("checked", PropValue::Bool(s.checked))
    }
    fn update(&self, msg: Self::Message, s: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            CheckBoxMessage::Toggle(v) if !s.disabled => s.checked = v,
            _ => {},
        }
    }
    fn measure(&self, _: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(
            150_f64.clamp(c.min_width, c.max_width),
            28_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 复选框盒子（方框）
        let box_size = 18.0_f64.min(bounds.size.height);
        let box_rect = Rect::new(
            bounds.origin.x + 4.0,
            bounds.origin.y + (bounds.size.height - box_size) * 0.5,
            box_size,
            box_size,
        );
        let box_color = if s.disabled {
            Color::new(0.4, 0.4, 0.4, 1.0)
        } else {
            Color::new(0.7, 0.7, 0.8, 1.0)
        };
        ctx.fill_rect(box_rect, box_color, 3.0);

        // 勾选标记（选中时绘制内部填充矩形）
        if s.checked && !s.disabled {
            let inner_pad = 4.0;
            ctx.fill_rect(
                Rect::new(
                    box_rect.origin.x + inner_pad,
                    box_rect.origin.y + inner_pad,
                    box_rect.size.width - inner_pad * 2.0,
                    box_rect.size.height - inner_pad * 2.0,
                ),
                Color::new(0.20, 0.55, 0.95, 1.0),
                2.0,
            );
        }

        // 标签文本
        let text_x = box_rect.origin.x + box_size + 8.0;
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
        AccessibilityNode::none()
            .label(s.label.as_str())
            .action(AccessibilityAction::Toggle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name() {
        assert_eq!(CheckBox.name(), "rgui_components::CheckBox");
    }
    #[test]
    fn toggle() {
        let mut s = CheckBoxState::new("x");
        CheckBox.update(
            CheckBoxMessage::Toggle(true),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.checked);
    }
    #[test]
    fn disabled() {
        let mut s = CheckBoxState::new("x").checked(true);
        s.disabled = true;
        CheckBox.update(
            CheckBoxMessage::Toggle(false),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.checked);
    }
    #[test]
    fn msg_name() {
        assert_eq!(CheckBoxMessage::Toggle(true).message_name(), "toggle");
    }

    #[test]
    fn paint_unchecked() {
        let state = CheckBoxState::new("Option");
        let bounds = Rect::new(0.0, 0.0, 150.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        CheckBox.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2, "应绘制方框 + 标签文本");
    }

    #[test]
    fn paint_checked() {
        let state = CheckBoxState::new("Option").checked(true);
        let bounds = Rect::new(0.0, 0.0, 150.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        CheckBox.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "选中状态应额外绘制勾选标记");
    }

    #[test]
    fn paint_disabled() {
        let mut state = CheckBoxState::new("Option");
        state.disabled = true;
        let bounds = Rect::new(0.0, 0.0, 150.0, 28.0);
        let mut ctx = PaintContext::new(bounds);
        CheckBox.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2);
    }
}
