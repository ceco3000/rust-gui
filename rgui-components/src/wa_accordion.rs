/// Translated from Web Awesome accordion
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
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-accordion 组件状态。
///
/// Accordion 是垂直堆叠的可展开面板容器，协调多个 `<wa-accordion-item>` 子组件。
/// 通过 `<slot>` 渲染子组件，自身无视觉绘制。
///
/// 简化项（Phase 0）：
/// - 模式协调逻辑（single/single-collapsible）→ 仅存 state，不做强制协调
/// - 键盘导航（ArrowUp/Down/Home/End）→ rgui 无焦点路由
/// - 子组件 prop 同步（syncIconPlacement/syncHeadingLevel/syncAppearance）
///   → paint_factory 中 prop 透传，不需要运行时 watch
/// - expandAll/collapseAll 程序化方法 → Phase 2
/// - SSR slot 变化处理 → rgui 无 hydration
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaAccordionState {
    /// 展开模式：single | single-collapsible | multiple
    pub mode: String,
    /// 子项展开/折叠图标位置：start | end
    pub icon_placement: String,
    /// 子项标题级别（1-6 或 none）
    pub heading_level: String,
    /// 视觉外观：filled | outlined | filled-outlined | plain
    pub appearance: String,
}

impl WaAccordionState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            mode: "multiple".into(),
            icon_placement: "end".into(),
            heading_level: "3".into(),
            appearance: "outlined".into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// Accordion 事件：
/// - `Expand` — 展开前（wa-expand，可取消）
/// - `AfterExpand` — 展开后
/// - `Collapse` — 折叠前（wa-collapse，可取消）
/// - `AfterCollapse` — 折叠后
///
/// Phase 0：所有事件无实际行为，保留占位供未来实现。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaAccordionMessage {
    Expand,
    AfterExpand,
    Collapse,
    AfterCollapse,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaAccordion;

impl WidgetSpec for WaAccordion {
    type State = WaAccordionState;
    type Message = WaAccordionMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaAccordion"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaAccordion")
            .prop("mode", PropValue::str(state.mode.as_str()))
            .prop("icon-placement", PropValue::str(state.icon_placement.as_str()))
            .prop("heading-level", PropValue::str(state.heading_level.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            // Phase 0: no coordination logic between items yet
            WaAccordionMessage::Expand => {}
            WaAccordionMessage::AfterExpand => {}
            WaAccordionMessage::Collapse => {}
            WaAccordionMessage::AfterCollapse => {}
        }
    }

    /// Accordion 是容器，尺寸由 Taffy 根据子节点和约束计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, _state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // Accordion 是纯容器，无自身视觉绘制。
        // 外观（border/background）由 AccordionItem 子组件各自渲染。
        // Phase 2 可添加容器级边框/背景。
        let _ = (bounds, ctx);
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::new(
            WidgetId::from_u64(0),
            AccessibilityRole::Group,
            Rect::ZERO,
        )
        .label("accordion")
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    fn make_ctx() -> ViewContext {
        ViewContext::new(Size::new(800.0, 600.0))
    }

    #[test]
    fn name() {
        assert_eq!(WaAccordion.name(), "rgui_components::WaAccordion");
    }

    #[test]
    fn default_state() {
        let s = WaAccordionState::new();
        assert_eq!(s.mode, "multiple");
        assert_eq!(s.icon_placement, "end");
        assert_eq!(s.heading_level, "3");
        assert_eq!(s.appearance, "outlined");
    }

    #[test]
    fn state_single_mode() {
        let s = WaAccordionState {
            mode: "single".into(),
            ..WaAccordionState::new()
        };
        assert_eq!(s.mode, "single");
    }

    #[test]
    fn state_single_collapsible_mode() {
        let s = WaAccordionState {
            mode: "single-collapsible".into(),
            ..WaAccordionState::new()
        };
        assert_eq!(s.mode, "single-collapsible");
    }

    #[test]
    fn state_icon_placement_start() {
        let s = WaAccordionState {
            icon_placement: "start".into(),
            ..WaAccordionState::new()
        };
        assert_eq!(s.icon_placement, "start");
    }

    #[test]
    fn state_appearance_filled() {
        let s = WaAccordionState {
            appearance: "filled".into(),
            ..WaAccordionState::new()
        };
        assert_eq!(s.appearance, "filled");
    }

    #[test]
    fn update_all_events_noop() {
        let mut s = WaAccordionState::new();
        let events = [
            WaAccordionMessage::Expand,
            WaAccordionMessage::AfterExpand,
            WaAccordionMessage::Collapse,
            WaAccordionMessage::AfterCollapse,
        ];
        for event in events {
            WaAccordion.update(event, &mut s, &mut UpdateContext::default());
        }
        // 不应 panic，状态不应改变
        assert_eq!(s.mode, "multiple");
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaAccordionState::new();
        let size = WaAccordion.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Accordion 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_produces_no_ops() {
        let s = WaAccordionState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordion.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "Accordion 无自身视觉绘制");
    }

    #[test]
    fn view_contains_props() {
        let s = WaAccordionState::new();
        let v = WaAccordion.view(&s, &make_ctx());
        assert!(v.props.contains_key("mode"));
        assert!(v.props.contains_key("icon-placement"));
        assert!(v.props.contains_key("heading-level"));
        assert!(v.props.contains_key("appearance"));
    }

    #[test]
    fn view_mode_prop() {
        let s = WaAccordionState {
            mode: "single".into(),
            ..WaAccordionState::new()
        };
        let v = WaAccordion.view(&s, &make_ctx());
        let val = v.props.get("mode").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "single"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaAccordionMessage::Expand.message_name(), "expand");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaAccordionState::schema_name(), "WaAccordionState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaAccordionState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaAccordionState>());
    }

    #[test]
    fn accessibility_label() {
        let s = WaAccordionState::new();
        let node = WaAccordion.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("accordion"));
    }
}
