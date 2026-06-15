//! Label 组件——纯文本标签。
//!
//! 用于显示只读文本，无交互行为。
//! 发送 `LabelMessage::NoOp`（占位消息）。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};

/// Label 业务状态。
///
/// 包含显示的文本内容。
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct LabelState {
    /// 标签显示的文本。
    pub text: String,
}

/// Label 消息类型（占位）。
///
/// Label 本身无交互行为，提供此枚举以满足 `WidgetSpec` 的关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum LabelMessage {
    NoOp,
}

/// Label 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。用于显示只读文本内容。
pub struct Label;
impl WidgetSpec for Label {
    type State = LabelState;
    type Message = LabelMessage;
    fn name(&self) -> &'static str {
        "rgui_components::Label"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("Label").prop("text", PropValue::str(s.text.as_str()))
    }
    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}
    fn measure(&self, s: &Self::State, _: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(8_f64 * s.text.len() as f64 + 8.0, 20.0)
    }
    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        ctx.draw_text(&s.text, bounds, Color::new(0.9, 0.9, 0.95, 1.0), 14.0);
    }
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(s.text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name() {
        assert_eq!(Label.name(), "rgui_components::Label");
    }
    #[test]
    fn view() {
        let v = Label.view(
            &LabelState { text: "Hi".into() },
            &ViewContext::new(Size::new(800.0, 600.0)),
        );
        assert!(v.props.contains_key("text"));
    }
    #[test]
    fn schema() {
        assert_eq!(LabelState::schema_name(), "LabelState");
    }

    #[test]
    fn paint_draws_text() {
        let state = LabelState {
            text: "Hello".into(),
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
        let mut ctx = PaintContext::new(bounds);
        Label.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 1);
    }
}
