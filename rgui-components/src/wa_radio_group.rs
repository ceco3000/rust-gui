/// Translated from Web Awesome wa-radio-group
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-radio-group 组件状态。
///
/// RadioGroup 是容器组件，管理多个 WaRadio 子项，
/// 提供统一的 label、hint 和分组语义。
/// 跳过 withLabel/withHint（SSR 专属属性）。
/// FormField trait impl 暂时跳过。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaRadioGroupState {
    /// 单选组的标签
    pub label: String,
    /// 提示文本
    pub hint: String,
    /// 表单 name 属性（用于提交）
    pub name: String,
    /// 禁用整个组及所有子 Radio
    pub disabled: bool,
    /// 子项排列方向：vertical | horizontal
    pub orientation: String,
    /// 当前选中的值
    pub value: String,
    /// 组尺寸，应用到所有子项
    pub size: String,
    /// 是否必填
    pub required: bool,
}

impl WaRadioGroupState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            orientation: "vertical".into(),
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaRadioGroupMessage {
    /// 选中值改变
    Change,
    /// 接收用户输入
    Input,
    /// 验证失败
    Invalid,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaRadioGroup;

impl WidgetSpec for WaRadioGroup {
    type State = WaRadioGroupState;
    type Message = WaRadioGroupMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaRadioGroup"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaRadioGroup")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("hint", PropValue::str(state.hint.as_str()))
            .prop("orientation", PropValue::str(state.orientation.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("required", PropValue::Bool(state.required))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("value", PropValue::str(state.value.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaRadioGroupMessage::Change | WaRadioGroupMessage::Input
            | WaRadioGroupMessage::Invalid => {}
        }
    }

    fn measure(&self, _state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // 容器组件：尺寸由 Taffy 根据子节点和约束决定
        Size::new(c.min_width, c.min_height)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let font_size: f32 = 14.0;
        let label_color = Color::new(0.1, 0.1, 0.1, 1.0);
        let hint_color = Color::new(0.5, 0.5, 0.5, 1.0);
        let gap: f64 = 4.0;

        // ── 绘制标签文本 ──
        if !state.label.is_empty() {
            let required_asterisk = if state.required { " *" } else { "" };
            let label_text = format!("{}{}", state.label, required_asterisk);
            let label_bounds = Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                font_size as f64 * 1.5,
            );
            ctx.draw_text(&label_text, label_bounds, label_color, font_size);
        }

        // ── 绘制提示文本 ──
        if !state.hint.is_empty() {
            let label_h = if state.label.is_empty() {
                0.0
            } else {
                font_size as f64 * 1.5 + gap
            };
            let hint_bounds = Rect::new(
                bounds.origin.x,
                bounds.origin.y + label_h,
                bounds.size.width,
                font_size as f64 * 1.2,
            );
            ctx.draw_text(&state.hint, hint_bounds, hint_color, font_size * 0.85);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(state.label.as_str())
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    fn name() {
        assert_eq!(WaRadioGroup.name(), "rgui_components::WaRadioGroup");
    }

    #[test]
    fn view_has_label() {
        let state = WaRadioGroupState::new("Choose one");
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_orientation() {
        let state = WaRadioGroupState::new("Settings");
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("orientation"),
            Some(&PropValue::Str("vertical".into()))
        );
    }

    #[test]
    fn view_horizontal_orientation() {
        let mut state = WaRadioGroupState::new("Options");
        state.orientation = "horizontal".into();
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("orientation"),
            Some(&PropValue::Str("horizontal".into()))
        );
    }

    #[test]
    fn view_required() {
        let mut state = WaRadioGroupState::new("Required");
        state.required = true;
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("required"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_disabled() {
        let mut state = WaRadioGroupState::new("Disabled Group");
        state.disabled = true;
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_has_hint() {
        let mut state = WaRadioGroupState::new("Pick one");
        state.hint = "Select a single option.".into();
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("hint"),
            Some(&PropValue::Str("Select a single option.".into()))
        );
    }

    #[test]
    fn view_has_value() {
        let mut state = WaRadioGroupState::new("Values");
        state.value = "radio-2".into();
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("value"), Some(&PropValue::Str("radio-2".into())));
    }

    #[test]
    fn view_has_size() {
        let mut state = WaRadioGroupState::new("Sizes");
        state.size = "l".into();
        let v = WaRadioGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("size"), Some(&PropValue::Str("l".into())));
    }

    #[test]
    fn update_change_is_handled() {
        let mut state = WaRadioGroupState::new("OK");
        WaRadioGroup.update(
            WaRadioGroupMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_input_is_handled() {
        let mut state = WaRadioGroupState::new("OK");
        WaRadioGroup.update(
            WaRadioGroupMessage::Input,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_invalid_is_handled() {
        let mut state = WaRadioGroupState::new("OK");
        WaRadioGroup.update(
            WaRadioGroupMessage::Invalid,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_min_constraints() {
        let state = WaRadioGroupState::new("Group");
        let size = WaRadioGroup.measure(
            &state,
            BoxConstraints::new(100.0, 500.0, 50.0, 300.0),
            &MeasureContext::default(),
        );
        assert_eq!(size.width, 100.0);
        assert_eq!(size.height, 50.0);
    }

    #[test]
    fn paint_with_label_produces_ops() {
        let state = WaRadioGroupState::new("My Group");
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaRadioGroup.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制标签文本");
    }

    #[test]
    fn paint_with_hint_produces_ops() {
        let mut state = WaRadioGroupState::new("Group");
        state.hint = "Select one.".into();
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaRadioGroup.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制提示文本");
    }

    #[test]
    fn paint_empty_label_no_ops() {
        let state = WaRadioGroupState::new("");
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaRadioGroup.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn accessibility_has_label() {
        let state = WaRadioGroupState::new("Accessibility Group");
        let node = WaRadioGroup.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Accessibility Group"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaRadioGroupMessage::Change.message_name(), "change");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaRadioGroupState::schema_name(), "WaRadioGroupState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaRadioGroupState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaRadioGroupState>());
    }
}
