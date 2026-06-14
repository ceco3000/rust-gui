//! TextField 组件——文本输入框。
//!
//! 支持文本编辑、占位符（placeholder）、禁用状态、焦点管理和光标定位。
//! 发送 [`TextFieldMessage`] 消息。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};

/// TextField 业务状态。
///
/// 包含当前文本、占位符、禁用标志、焦点状态和光标位置。
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct TextFieldState {
    /// 当前输入的文本内容。
    pub text: String,
    /// 占位符文本（输入框为空时显示）。
    pub placeholder: String,
    /// 是否禁用。
    pub disabled: bool,
    /// 是否获得焦点。
    pub focused: bool,
    /// 光标位置（字符索引）。
    pub cursor_position: usize,
}
impl TextFieldState {
    /// 创建新的 TextFieldState，指定占位符文本。
    #[must_use]
    pub fn new(p: impl Into<String>) -> Self {
        Self {
            placeholder: p.into(),
            ..Self::default()
        }
    }
}

/// TextField 消息类型。
///
/// - `TextChanged(String)`: 文本内容改变
/// - `FocusIn` / `FocusOut`: 焦点变化
/// - `CursorMoved(usize)`: 光标位置移动
/// - `Submitted`: 提交（回车）
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum TextFieldMessage {
    TextChanged(String),
    FocusIn,
    FocusOut,
    CursorMoved(usize),
    Submitted,
}

/// TextField 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。用于单行文本输入场景。
pub struct TextField;
impl WidgetSpec for TextField {
    type State = TextFieldState;
    type Message = TextFieldMessage;
    fn name(&self) -> &'static str {
        "rgui_components::TextField"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("TextField")
            .prop("text", PropValue::str(s.text.as_str()))
            .prop("placeholder", PropValue::str(s.placeholder.as_str()))
            .prop("disabled", PropValue::Bool(s.disabled))
            .prop("focused", PropValue::Bool(s.focused))
    }
    fn update(&self, msg: Self::Message, s: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            TextFieldMessage::TextChanged(t) => {
                s.text = t;
                s.cursor_position = s.text.len();
            },
            TextFieldMessage::FocusIn => s.focused = true,
            TextFieldMessage::FocusOut => s.focused = false,
            TextFieldMessage::CursorMoved(p) => s.cursor_position = p.min(s.text.len()),
            TextFieldMessage::Submitted => {},
        }
    }
    fn measure(&self, _: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(
            200_f64.clamp(c.min_width, c.max_width),
            32_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(s.text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name() {
        assert_eq!(TextField.name(), "rgui_components::TextField");
    }
    #[test]
    fn text_update() {
        let mut s = TextFieldState::new("...");
        TextField.update(
            TextFieldMessage::TextChanged("hi".into()),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert_eq!(s.text, "hi");
        assert_eq!(s.cursor_position, 2);
    }
    #[test]
    fn focus() {
        let mut s = TextFieldState::new("...");
        TextField.update(
            TextFieldMessage::FocusIn,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.focused);
    }
    #[test]
    fn measure() {
        let s = TextFieldState::new("...");
        let sz = TextField.measure(
            &s,
            BoxConstraints::new(100.0, 400.0, 24.0, 48.0),
            &MeasureContext::default(),
        );
        assert!(sz.width >= 100.0);
    }
    #[test]
    fn placeholder() {
        let v = TextField.view(
            &TextFieldState::new("搜索..."),
            &ViewContext::new(Size::new(800.0, 600.0)),
        );
        assert!(v.props.contains_key("placeholder"));
    }
}
