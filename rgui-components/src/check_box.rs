//! CheckBox 组件——复选框。
//!
//! 支持选中/未选中状态切换、禁用状态。
//! 发送 `CheckBoxMessage` 消息。

use rgui_core::a11y::{AccessibilityAction, AccessibilityNode};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
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
    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}
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
}
