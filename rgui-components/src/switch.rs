use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct SwitchState {
    pub on: bool,
    pub disabled: bool,
    pub label: String,
}
impl SwitchState {
    #[must_use]
    pub fn new(l: impl Into<String>) -> Self {
        Self {
            label: l.into(),
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum SwitchMessage {
    Toggle(bool),
    FocusIn,
    FocusOut,
}
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
    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}
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
}
