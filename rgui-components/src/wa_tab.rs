/// Translated from Web Awesome wa-tab
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::WidgetId;
use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
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

/// Web Awesome wa-tab 组件状态。
///
/// Tab 是标签页导航中的单个选项卡，点击可切换到对应的面板。
/// 当前阶段 0：静态渲染，无交互事件。
///
/// 简化项：
/// - 键盘导航跳过（TabGroup 集中处理）
/// - @query + tabIndex 跳过（rgui 无 DOM）
/// - slot + role 为内部属性，不暴露到 state
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTabState {
    /// 关联的面板名称，与 WaTabPanel.name 对应。
    pub panel: String,
    /// 是否为当前激活的标签页。
    pub active: bool,
    /// 禁用状态。
    pub disabled: bool,
    /// 选项卡显示的标签文本（来自 slot 内容）。
    pub label: String,
}

impl WaTabState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            panel: String::new(),
            active: false,
            disabled: false,
            label: String::new(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// 标签页无独立事件（TabGroup 集中处理交互），占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTabMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaTab;

impl WidgetSpec for WaTab {
    type State = WaTabState;
    type Message = WaTabMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTab"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaTab")
            .prop("panel", PropValue::str(state.panel.as_str()))
            .prop("label", PropValue::str(if state.label.is_empty() { "Tab" } else { state.label.as_str() }))
            .prop("active", PropValue::Bool(state.active))
            .prop("disabled", PropValue::Bool(state.disabled))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTabMessage::NoOp => {},
        }
    }

    /// Tab 是叶子组件，尺寸由 Taffy 布局决定。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let h: f64 = bounds.size.height;
        let w: f64 = bounds.size.width;

        if h < 8.0 || w < 8.0 {
            return;
        }

        // 根据状态选择颜色
        let bg = if state.active {
            Color::new(1.0, 1.0, 1.0, 1.0) // active: white
        } else {
            Color::new(0.95, 0.95, 0.95, 1.0) // inactive: light gray
        };

        let text_color = if state.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0) // disabled: gray text
        } else if state.active {
            Color::new(0.06, 0.41, 0.91, 1.0) // active: blue text (~ #0E69E9)
        } else {
            Color::new(0.2, 0.2, 0.2, 1.0) // normal: dark text
        };

        // 底部激活指示线（active 时绘制）
        let indicator_height: f64 = 3.0;
        if state.active {
            let indicator_color = Color::new(0.06, 0.41, 0.91, 1.0);
            let indicator_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y + h - indicator_height,
                w,
                indicator_height,
            );
            ctx.fill_rect(indicator_rect, indicator_color, 0.0);
        }

        // 背景填充
        ctx.fill_rect(bounds, bg, 0.0);

        // 标签文本
        let label = if state.label.is_empty() { "Tab" } else { state.label.as_str() };
        let font_size: f32 = (h * 0.44) as f32;
        let text_rect = Rect::new(
            bounds.origin.x + 8.0,
            bounds.origin.y,
            w - 16.0,
            h - indicator_height,
        );
        ctx.draw_text(label, text_rect, text_color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label_text = if state.label.is_empty() {
            "tab"
        } else {
            state.label.as_str()
        };

        AccessibilityNode::new(
            WidgetId::from_u64(0),
            AccessibilityRole::Tab,
            Rect::ZERO,
        )
        .label(label_text)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::geometry::{BoxConstraints, Rect, Size};

    #[test]
    fn default_state() {
        let state = WaTabState::new();
        assert_eq!(state.panel, "");
        assert!(!state.active);
        assert!(!state.disabled);
        assert_eq!(state.label, "");
    }

    #[test]
    fn state_active_with_panel() {
        let state = WaTabState {
            panel: "general".into(),
            active: true,
            disabled: false,
            label: "General".into(),
        };
        assert_eq!(state.panel, "general");
        assert!(state.active);
        assert!(!state.disabled);
        assert_eq!(state.label, "General");
    }

    #[test]
    fn state_disabled() {
        let state = WaTabState {
            disabled: true,
            ..WaTabState::new()
        };
        assert!(state.disabled);
        assert!(!state.active);
    }

    #[test]
    fn message_noop() {
        let mut state = WaTabState::new();
        let mut ctx = UpdateContext::default();
        WaTab.update(WaTabMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.panel, "");
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaTabState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaTab.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_active_produces_fill_and_text() {
        let state = WaTabState {
            active: true,
            label: "Active".into(),
            ..WaTabState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaTab.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(!ops.is_empty(), "active Tab 应产生绘制操作");

        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 2, "active Tab 应有 2 个 FillRect（背景+指示线）");

        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 1, "应有 1 个 DrawText（标签）");
    }

    #[test]
    fn paint_inactive_produces_fill_and_text() {
        let state = WaTabState {
            active: false,
            label: "Inactive".into(),
            ..WaTabState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaTab.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(!ops.is_empty());

        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 1, "inactive Tab 应有 1 个 FillRect（背景，无指示线）");

        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 1, "应有 1 个 DrawText（标签）");
    }

    #[test]
    fn paint_disabled_produces_ops() {
        let state = WaTabState {
            disabled: true,
            label: "Disabled".into(),
            ..WaTabState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 120.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaTab.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 1, "disabled Tab 也应绘制标签文本");
    }

    #[test]
    fn paint_too_small_bounds_returns_early() {
        let state = WaTabState {
            active: true,
            ..WaTabState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 3.0, 3.0);
        let mut ctx = PaintContext::new(bounds);
        WaTab.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "过小 bounds 应提前返回");
    }

    #[test]
    fn accessibility_default_state() {
        let state = WaTabState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaTab.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("tab"));
    }

    #[test]
    fn accessibility_with_label() {
        let state = WaTabState {
            label: "Settings".into(),
            active: true,
            ..WaTabState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaTab.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("Settings"));
    }

    #[test]
    fn view_contains_props() {
        let state = WaTabState {
            panel: "settings-panel".into(),
            active: true,
            disabled: false,
            label: "Settings".into(),
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaTab.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaTab");

        if let Some(PropValue::Bool(a)) = view.props.get("active") {
            assert!(*a);
        } else {
            panic!("Expected Bool prop 'active'");
        }

        if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
            assert!(!*d);
        } else {
            panic!("Expected Bool prop 'disabled'");
        }

        if let Some(PropValue::Str(s)) = view.props.get("panel") {
            assert_eq!(s.as_ref(), "settings-panel");
        } else {
            panic!("Expected Str prop 'panel'");
        }
    }

    #[test]
    fn view_empty_label_defaults() {
        let state = WaTabState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaTab.view(&state, &ctx);
        if let Some(PropValue::Str(s)) = view.props.get("label") {
            assert_eq!(s.as_ref(), "Tab");
        }
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaTab.name(), "rgui_components::WaTab");
    }
}
