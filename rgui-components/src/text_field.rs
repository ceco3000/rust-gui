use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct TextFieldState {
    pub text: String,
    pub placeholder: String,
    pub disabled: bool,
    pub focused: bool,
    pub cursor_position: usize,
}
impl TextFieldState {
    #[must_use]
    pub fn new(p: impl Into<String>) -> Self {
        Self {
            placeholder: p.into(),
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum TextFieldMessage {
    TextChanged(String),
    FocusIn,
    FocusOut,
    CursorMoved(usize),
    Submitted,
}
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
