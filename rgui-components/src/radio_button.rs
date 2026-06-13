use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct RadioButtonState {
    pub label: String,
    pub selected: bool,
    pub disabled: bool,
    pub group: String,
}
impl RadioButtonState {
    #[must_use]
    pub fn new(label: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            group: group.into(),
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum RadioButtonMessage {
    Select,
    FocusIn,
    FocusOut,
}
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
    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}
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
}
