/// Translated from Web Awesome wa-checkbox-group
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

/// Web Awesome wa-checkbox-group 组件状态。
///
/// CheckboxGroup 是容器组件，管理多个 Checkbox/Switch 子项，
/// 提供统一的 label、hint 和分组语义。
/// 跳过 withLabel/withHint（SSR 专属属性）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaCheckboxGroupState {
    /// 复选框组的标签
    pub label: String,
    /// 提示文本
    pub hint: String,
    /// 子项排列方向：vertical | horizontal
    pub orientation: String,
    /// 组尺寸，应用到所有子项
    pub size: String,
    /// 是否必填（仅视觉指示）
    pub required: bool,
}

impl WaCheckboxGroupState {
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

/// CheckboxGroup 无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaCheckboxGroupMessage {
    /// 占位消息——CheckboxGroup 无交互事件
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaCheckboxGroup;

impl WidgetSpec for WaCheckboxGroup {
    type State = WaCheckboxGroupState;
    type Message = WaCheckboxGroupMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaCheckboxGroup"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaCheckboxGroup")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("hint", PropValue::str(state.hint.as_str()))
            .prop("orientation", PropValue::str(state.orientation.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("required", PropValue::Bool(state.required))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaCheckboxGroupMessage::NoOp => {},
        }
    }

    fn measure(&self, _state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // 容器组件：尺寸由 Taffy 根据子节点和约束决定
        // 返回最小约束，让 Taffy 自由伸缩
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
        assert_eq!(WaCheckboxGroup.name(), "rgui_components::WaCheckboxGroup");
    }

    #[test]
    fn view_has_label() {
        let state = WaCheckboxGroupState::new("Preferences");
        let v = WaCheckboxGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_orientation() {
        let state = WaCheckboxGroupState::new("Settings");
        let v = WaCheckboxGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("orientation"),
            Some(&PropValue::Str("vertical".into()))
        );
    }

    #[test]
    fn view_horizontal_orientation() {
        let mut state = WaCheckboxGroupState::new("Options");
        state.orientation = "horizontal".into();
        let v = WaCheckboxGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("orientation"),
            Some(&PropValue::Str("horizontal".into()))
        );
    }

    #[test]
    fn view_required() {
        let mut state = WaCheckboxGroupState::new("Required");
        state.required = true;
        let v = WaCheckboxGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("required"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_has_hint() {
        let mut state = WaCheckboxGroupState::new("Toppings");
        state.hint = "Choose as many as you like.".into();
        let v = WaCheckboxGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("hint"),
            Some(&PropValue::Str("Choose as many as you like.".into()))
        );
    }

    #[test]
    fn view_has_size() {
        let mut state = WaCheckboxGroupState::new("Sizes");
        state.size = "l".into();
        let v = WaCheckboxGroup.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("size"), Some(&PropValue::Str("l".into())));
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaCheckboxGroupState::new("OK");
        WaCheckboxGroup.update(
            WaCheckboxGroupMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_min_constraints() {
        let state = WaCheckboxGroupState::new("Group");
        let size = WaCheckboxGroup.measure(
            &state,
            BoxConstraints::new(100.0, 500.0, 50.0, 300.0),
            &MeasureContext::default(),
        );
        assert_eq!(size.width, 100.0);
        assert_eq!(size.height, 50.0);
    }

    #[test]
    fn paint_with_label_produces_ops() {
        let state = WaCheckboxGroupState::new("My Group");
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaCheckboxGroup.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制标签文本");
    }

    #[test]
    fn paint_with_hint_produces_ops() {
        let mut state = WaCheckboxGroupState::new("Group");
        state.hint = "Select one or more.".into();
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaCheckboxGroup.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "应至少绘制提示文本");
    }

    #[test]
    fn paint_empty_label_no_ops() {
        let state = WaCheckboxGroupState::new("");
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaCheckboxGroup.paint(&state, bounds, &mut ctx);
        // 无标签无提示 → 无绘制操作
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn accessibility_has_label() {
        let state = WaCheckboxGroupState::new("Accessibility Group");
        let node = WaCheckboxGroup.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Accessibility Group"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaCheckboxGroupMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaCheckboxGroupState::schema_name(), "WaCheckboxGroupState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaCheckboxGroupState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaCheckboxGroupState>());
    }
}
