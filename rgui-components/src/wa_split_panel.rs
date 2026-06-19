/// Translated from Web Awesome wa-split-panel
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

/// Web Awesome wa-split-panel 组件状态。
///
/// SplitPanel 将两个面板用可拖拽分隔条分割，让用户调整两侧尺寸。
/// Phase 0：仅支持静态分隔条位置，暂不支持拖拽手势。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaSplitPanelState {
    /// 分隔条当前位置（0-100 百分比，默认 50）
    pub position: f64,
    /// 方向：horizontal | vertical
    pub orientation: String,
    /// 是否禁用拖拽
    pub disabled: bool,
    /// 主面板：None | "start" | "end"
    pub primary: Option<String>,
}

impl WaSplitPanelState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: 50.0,
            orientation: "horizontal".into(),
            disabled: false,
            primary: None,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaSplitPanelMessage {
    /// 分隔条位置改变时发出
    Reposition,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaSplitPanel;

impl WidgetSpec for WaSplitPanel {
    type State = WaSplitPanelState;
    type Message = WaSplitPanelMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaSplitPanel"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaSplitPanel")
            .prop("orientation", PropValue::str(state.orientation.as_str()))
            .prop(
                "position",
                PropValue::Float(ordered_float::OrderedFloat(state.position)),
            )
            .prop("disabled", PropValue::Bool(state.disabled));
        if let Some(ref primary) = state.primary {
            v = v.prop("primary", PropValue::str(primary.as_str()));
        }
        v
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaSplitPanelMessage::Reposition => {
                // Phase 0: reposition event handled by external state update
            },
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        // SplitPanel 是容器，尺寸由 Taffy 根据子节点和约束计算
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let divider_width: f64 = 4.0;
        let divider_color = Color::new(0.82, 0.82, 0.84, 1.0); // surface-border

        let x = bounds.origin.x;
        let y = bounds.origin.y;
        let w = bounds.size.width;
        let h = bounds.size.height;

        if state.orientation == "vertical" {
            // 水平分隔条
            let divider_y = y + (state.position / 100.0) * h - divider_width / 2.0;
            let clamped_y = divider_y.max(y).min(y + h - divider_width);
            ctx.fill_rect(
                Rect::new(x, clamped_y, w, divider_width),
                divider_color,
                0.0,
            );
        } else {
            // 垂直分隔条（horizontal orientation，默认）
            let divider_x = x + (state.position / 100.0) * w - divider_width / 2.0;
            let clamped_x = divider_x.max(x).min(x + w - divider_width);
            ctx.fill_rect(
                Rect::new(clamped_x, y, divider_width, h),
                divider_color,
                0.0,
            );
        }
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("split panel")
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
        assert_eq!(WaSplitPanel.name(), "rgui_components::WaSplitPanel");
    }

    #[test]
    fn view_has_orientation() {
        let state = WaSplitPanelState::new();
        let v = WaSplitPanel.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("orientation"));
    }

    #[test]
    fn view_default_horizontal() {
        let state = WaSplitPanelState::new();
        let v = WaSplitPanel.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("orientation").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "horizontal"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_default_position_50() {
        let state = WaSplitPanelState::new();
        let v = WaSplitPanel.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Float(f) => assert!((f.0 - 50.0).abs() < 0.01),
            _ => panic!("expected Float prop"),
        }
    }

    #[test]
    fn view_disabled_false_by_default() {
        let state = WaSplitPanelState::new();
        let v = WaSplitPanel.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("disabled").unwrap();
        match val {
            PropValue::Bool(b) => assert!(!b),
            _ => panic!("expected Bool prop"),
        }
    }

    #[test]
    fn view_with_primary() {
        let state = WaSplitPanelState {
            primary: Some("start".into()),
            ..WaSplitPanelState::new()
        };
        let v = WaSplitPanel.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("primary"));
    }

    #[test]
    fn view_vertical_orientation() {
        let state = WaSplitPanelState {
            orientation: "vertical".into(),
            ..WaSplitPanelState::new()
        };
        let v = WaSplitPanel.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("orientation").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "vertical"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_reposition_does_not_panic() {
        let mut state = WaSplitPanelState::new();
        WaSplitPanel.update(
            WaSplitPanelMessage::Reposition,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_zero_delegating_to_layout() {
        let state = WaSplitPanelState::new();
        let size = WaSplitPanel.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "SplitPanel 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_horizontal_produces_divider() {
        let state = WaSplitPanelState::new(); // default: horizontal, position=50
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaSplitPanel.paint(&state, bounds, &mut ctx);
        // 垂直分隔条：1 个 fill_rect
        assert_eq!(ctx.op_count(), 1, "horizontal SplitPanel 应绘制 1 条分隔线");
    }

    #[test]
    fn paint_vertical_produces_divider() {
        let state = WaSplitPanelState {
            orientation: "vertical".into(),
            ..WaSplitPanelState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaSplitPanel.paint(&state, bounds, &mut ctx);
        // 水平分隔条：1 个 fill_rect
        assert_eq!(ctx.op_count(), 1, "vertical SplitPanel 应绘制 1 条分隔线");
    }

    #[test]
    fn paint_position_25_left_side() {
        let state = WaSplitPanelState {
            position: 25.0,
            ..WaSplitPanelState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaSplitPanel.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 1);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaSplitPanelMessage::Reposition.message_name(), "reposition");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaSplitPanelState::schema_name(), "WaSplitPanelState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaSplitPanelState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaSplitPanelState>());
    }

    #[test]
    fn accessibility_label() {
        let state = WaSplitPanelState::new();
        let node = WaSplitPanel.accessibility(&state, &AccessContext::new(Rect::ZERO));
        // AccessibilityNode::none().label("split panel")
        assert!(node.label.is_some());
    }
}
