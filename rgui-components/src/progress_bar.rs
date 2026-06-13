use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct ProgressBarState {
    pub value: f64,
    pub label: String,
}
impl ProgressBarState {
    #[must_use]
    pub fn new(v: f64) -> Self {
        Self {
            value: v.clamp(0.0, 1.0),
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum ProgressBarMessage {
    NoOp,
}
pub struct ProgressBar;
impl WidgetSpec for ProgressBar {
    type State = ProgressBarState;
    type Message = ProgressBarMessage;
    fn name(&self) -> &'static str {
        "rgui_components::ProgressBar"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("ProgressBar").prop(
            "percent",
            PropValue::Float(ordered_float::OrderedFloat(
                (s.value * 100.0).clamp(0.0, 100.0),
            )),
        )
    }
    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}
    fn measure(&self, _: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(
            200_f64.clamp(c.min_width, c.max_width),
            20_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, _: &Self::State, _: Rect, _: &mut PaintContext) {}
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(format!("{:.0}%", s.value * 100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamped() {
        let s = ProgressBarState::new(1.5);
        assert!((s.value - 1.0).abs() < f64::EPSILON);
    }
    #[test]
    fn view_pct() {
        let v = ProgressBar.view(
            &ProgressBarState::new(0.75),
            &ViewContext::new(Size::new(800.0, 600.0)),
        );
        assert!(v.props.contains_key("percent"));
    }
}
