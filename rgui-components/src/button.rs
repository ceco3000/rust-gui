//! Button 组件——触发操作的可点击按钮。
//!
//! 支持标签文本、禁用状态和按压状态。
//! 发送 [`ButtonMessage`] 消息。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// Button 业务状态。
///
/// 包含按钮的标签文本、禁用标志和按压状态。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct ButtonState {
    /// 按钮标签文本。
    pub label: String,
    /// 是否禁用（禁用状态下不响应按压事件）。
    pub disabled: bool,
    /// 是否处于按压中状态。
    pub pressed: bool,
}
impl ButtonState {
    /// 创建新的 ButtonState，指定标签文本。
    #[must_use]
    pub fn new(l: impl Into<String>) -> Self {
        Self {
            label: l.into(),
            ..Self::default()
        }
    }
    /// 设置禁用状态（builder 风格）。
    #[must_use]
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
}

/// Button 消息类型。
///
/// - `Clicked`: 完整点击完成（按下并释放）
/// - `Pressed`: 按钮被按下
/// - `Released`: 按钮被释放
/// - `FocusGained`: 获得焦点
/// - `FocusLost`: 失去焦点
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum ButtonMessage {
    Clicked,
    Pressed,
    Released,
    FocusGained,
    FocusLost,
}

/// Button 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。可通过 `ui!` 宏在声明式 UI 中使用。
pub struct Button;
impl WidgetSpec for Button {
    type State = ButtonState;
    type Message = ButtonMessage;
    fn name(&self) -> &'static str {
        "rgui_components::Button"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::Button")
            .prop("label", PropValue::str(s.label.as_str()))
            .prop("disabled", PropValue::Bool(s.disabled))
    }
    fn update(&self, msg: Self::Message, s: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            ButtonMessage::Pressed if !s.disabled => s.pressed = true,
            ButtonMessage::Released | ButtonMessage::FocusLost if !s.disabled => s.pressed = false,
            _ => {},
        }
    }
    fn measure(&self, s: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let tw = s.label.len() as f64 * 14.0 * 0.6;
        Size::new(
            (tw + 32.0).max(64.0).clamp(c.min_width, c.max_width),
            32_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(s.label.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name() {
        assert_eq!(Button.name(), "rgui_components::Button");
    }
    #[test]
    fn view() {
        let v = Button.view(
            &ButtonState::new("OK"),
            &ViewContext::new(Size::new(800.0, 600.0)),
        );
        assert!(v.props.contains_key("label"));
    }
    #[test]
    fn pressed() {
        let mut s = ButtonState::new("OK");
        Button.update(
            ButtonMessage::Pressed,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.pressed);
    }
    #[test]
    fn disabled() {
        let mut s = ButtonState::new("OK").disabled(true);
        Button.update(
            ButtonMessage::Pressed,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.pressed);
    }
    #[test]
    fn derive_msg() {
        assert_eq!(ButtonMessage::Clicked.message_name(), "clicked");
    }
    #[test]
    fn derive_state() {
        assert_eq!(ButtonState::schema_name(), "ButtonState");
    }
}
