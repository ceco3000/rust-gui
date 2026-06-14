//! Switch 组件——开关切换。
//!
//! 支持开/关两种状态的切换、禁用状态。
//! 发送 `SwitchMessage` 消息。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};

/// Switch 业务状态。
///
/// 包含开关状态、禁用标志和标签文本。
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct SwitchState {
    /// 是否处于打开状态。
    pub on: bool,
    /// 是否禁用。
    pub disabled: bool,
    /// 开关标签文本。
    pub label: String,
}
impl SwitchState {
    /// 创建新的 SwitchState，指定标签文本。
    #[must_use]
    pub fn new(l: impl Into<String>) -> Self {
        Self {
            label: l.into(),
            ..Self::default()
        }
    }
}

/// Switch 消息类型。
///
/// - `Toggle(bool)`: 切换到指定状态
/// - `FocusIn` / `FocusOut`: 焦点变化
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum SwitchMessage {
    Toggle(bool),
    FocusIn,
    FocusOut,
}

/// Switch 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。用于二元开关场景。
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
