/// Translated from Web Awesome wa-tab-panel
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

/// Web Awesome wa-tab-panel 组件状态。
///
/// TabPanel 持有单个标签页的内容，位于 TabGroup 内部。
/// 当前阶段 0：静态渲染，active 面板显示浅色背景。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTabPanelState {
    /// 面板名称，与 Tab 的 panel 属性对应。
    pub name: String,
    /// 是否为当前激活的面板。
    pub active: bool,
}

impl WaTabPanelState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            active: false,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// 标签面板无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTabPanelMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaTabPanel;

impl WidgetSpec for WaTabPanel {
    type State = WaTabPanelState;
    type Message = WaTabPanelMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTabPanel"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaTabPanel")
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("active", PropValue::Bool(state.active))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTabPanelMessage::NoOp => {},
        }
    }

    /// TabPanel 是容器组件，尺寸由 Taffy 布局决定。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if state.active {
            // 激活面板：白色背景 + 浅灰边框
            let bg = Color::new(1.0, 1.0, 1.0, 1.0);
            ctx.fill_rect(bounds, bg, 0.0);
        }
        // 非激活面板不绘制（被 TabGroup 隐藏）
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label_text = if state.name.is_empty() {
            "tab panel"
        } else {
            state.name.as_str()
        };

        AccessibilityNode::new(
            WidgetId::from_u64(0),
            AccessibilityRole::TabPanel,
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
        let state = WaTabPanelState::new();
        assert_eq!(state.name, "");
        assert!(!state.active);
    }

    #[test]
    fn state_with_name() {
        let state = WaTabPanelState {
            name: "tab-1".into(),
            active: true,
        };
        assert_eq!(state.name, "tab-1");
        assert!(state.active);
    }

    #[test]
    fn state_inactive() {
        let state = WaTabPanelState {
            active: false,
            ..WaTabPanelState::new()
        };
        assert!(!state.active);
    }

    #[test]
    fn message_noop() {
        let mut state = WaTabPanelState::new();
        let mut ctx = UpdateContext::default();
        WaTabPanel.update(WaTabPanelMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.name, "");
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaTabPanelState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaTabPanel.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_active_produces_fill() {
        let state = WaTabPanelState {
            active: true,
            ..WaTabPanelState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        WaTabPanel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(!ops.is_empty(), "active TabPanel 应产生绘制操作");
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 1, "应有恰好 1 个 FillRect");
    }

    #[test]
    fn paint_inactive_produces_nothing() {
        let state = WaTabPanelState {
            active: false,
            ..WaTabPanelState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        WaTabPanel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "inactive TabPanel 不应产生绘制操作");
    }

    #[test]
    fn accessibility_default_state() {
        let state = WaTabPanelState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaTabPanel.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("tab panel"));
    }

    #[test]
    fn accessibility_with_name() {
        let state = WaTabPanelState {
            name: "settings".into(),
            active: true,
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaTabPanel.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("settings"));
    }

    #[test]
    fn view_contains_props() {
        let state = WaTabPanelState {
            name: "general".into(),
            active: true,
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaTabPanel.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaTabPanel");

        if let Some(PropValue::Bool(a)) = view.props.get("active") {
            assert!(*a);
        } else {
            panic!("Expected Bool prop 'active'");
        }

        if let Some(PropValue::Str(s)) = view.props.get("name") {
            assert_eq!(s.as_ref(), "general");
        } else {
            panic!("Expected Str prop 'name'");
        }
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaTabPanel.name(), "rgui_components::WaTabPanel");
    }
}
