use rgui_core::a11y::{AccessibilityAction, AccessibilityNode};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};
use std::sync::Arc;
#[derive(Debug, Clone, serde::Serialize, PersistState)]
pub struct SliderState {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub disabled: bool,
    pub dragging: bool,
}
impl Default for SliderState {
    fn default() -> Self {
        Self {
            value: 50.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            disabled: false,
            dragging: false,
        }
    }
}
impl SliderState {
    #[must_use]
    pub fn new(v: f64, min: f64, max: f64) -> Self {
        Self {
            value: v.clamp(min, max),
            min,
            max,
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum SliderMessage {
    ValueChanged(f64),
    DragStarted,
    DragEnded,
}
pub struct Slider;
impl WidgetSpec for Slider {
    type State = SliderState;
    type Message = SliderMessage;
    fn name(&self) -> &'static str {
        "rgui_components::Slider"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let pct = ((s.value - s.min) / (s.max - s.min) * 100.0).clamp(0.0, 100.0);
        WidgetView::new("Slider")
            .prop(
                "percent",
                PropValue::Float(ordered_float::OrderedFloat(pct)),
            )
            .prop("disabled", PropValue::Bool(s.disabled))
    }
    fn update(&self, msg: Self::Message, s: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            SliderMessage::ValueChanged(v) if !s.disabled => {
                s.value = ((v / s.step).round() * s.step).clamp(s.min, s.max)
            },
            SliderMessage::DragStarted if !s.disabled => s.dragging = true,
            SliderMessage::DragEnded => s.dragging = false,
            _ => {},
        }
    }
    fn measure(&self, _: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(
            200_f64.clamp(c.min_width, c.max_width),
            24_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
            .label(format!("{}", s.value))
            .action(AccessibilityAction::SetValue(Arc::from(format!(
                "{}",
                s.value
            ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn value_change() {
        let mut s = SliderState::new(50.0, 0.0, 100.0);
        Slider.update(
            SliderMessage::ValueChanged(75.0),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert_eq!(s.value, 75.0);
    }
    #[test]
    fn clamp() {
        let mut s = SliderState::new(50.0, 0.0, 100.0);
        Slider.update(
            SliderMessage::ValueChanged(150.0),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert_eq!(s.value, 100.0);
    }
    #[test]
    fn disabled() {
        let mut s = SliderState::new(50.0, 0.0, 100.0);
        s.disabled = true;
        Slider.update(
            SliderMessage::ValueChanged(75.0),
            &mut s,
            &mut UpdateContext::default(),
        );
        assert_eq!(s.value, 50.0);
    }
    #[test]
    fn drag() {
        let mut s = SliderState::default();
        Slider.update(
            SliderMessage::DragStarted,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.dragging);
        Slider.update(
            SliderMessage::DragEnded,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.dragging);
    }
    #[test]
    fn view_pct() {
        let v = Slider.view(
            &SliderState::new(25.0, 0.0, 100.0),
            &ViewContext::new(Size::new(800.0, 600.0)),
        );
        assert!(v.props.contains_key("percent"));
    }
}
