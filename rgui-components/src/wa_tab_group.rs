/// Translated from Web Awesome wa-tab-group
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

/// Web Awesome wa-tab-group 组件状态。
///
/// TabGroup 将标签页组织到单一容器中，一次显示一个面板，通过标签切换。
/// 当前阶段 0：静态渲染，无交互事件处理。仅绘制导航区域背景 + 内容区域。
///
/// 简化项：
/// - 滚动控制按钮跳过（Phase 0 无滚动检测）
/// - 键盘导航（ArrowLeft/Right/Home/End）跳过
/// - MutationObserver / ResizeObserver 跳过（rgui 无 DOM）
/// - rtl 方向跳过
/// - scrollIntoView 跳过
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTabGroupState {
    /// 当前激活的面板名称（对应 WaTab.panel / WaTabPanel.name）。
    pub active: String,
    /// 标签位置：top | bottom | start | end。
    pub placement: String,
    /// 键盘激活模式：auto（即时切换）| manual（聚焦后回车/空格切换）。
    pub activation: String,
    /// 是否禁用溢出滚动按钮。
    pub without_scroll_controls: bool,
}

impl WaTabGroupState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: String::new(),
            placement: "top".into(),
            activation: "auto".into(),
            without_scroll_controls: false,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTabGroupMessage {
    /// 标签页切换——显示新面板（wa-tab-show 事件）。
    TabShow,
    /// 标签页切换——隐藏旧面板（wa-tab-hide 事件）。
    TabHide,
    /// 滚动到起始位置。
    ScrollToStart,
    /// 滚动到结束位置。
    ScrollToEnd,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaTabGroup;

impl WidgetSpec for WaTabGroup {
    type State = WaTabGroupState;
    type Message = WaTabGroupMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTabGroup"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaTabGroup")
            .prop("active", PropValue::str(state.active.as_str()))
            .prop("placement", PropValue::str(state.placement.as_str()))
            .prop("activation", PropValue::str(state.activation.as_str()))
            .prop(
                "without-scroll-controls",
                PropValue::Bool(state.without_scroll_controls),
            )
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTabGroupMessage::TabShow => {
                // Phase 0：静态渲染，不做动态切换
            },
            WaTabGroupMessage::TabHide => {
                // Phase 0：静态渲染，不做动态切换
            },
            WaTabGroupMessage::ScrollToStart => {
                // Phase 0：跳过滚动
            },
            WaTabGroupMessage::ScrollToEnd => {
                // Phase 0：跳过滚动
            },
        }
    }

    /// TabGroup 是容器组件，尺寸由 Taffy 布局（子节点驱动）。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let h: f64 = bounds.size.height;
        let w: f64 = bounds.size.width;

        if h < 8.0 || w < 8.0 {
            return;
        }

        // 导航栏高度（约 44px 用于容纳标签）
        let nav_height: f64 = 44.0;
        let is_top = state.placement == "top" || state.placement.is_empty();
        let is_bottom = state.placement == "bottom";

        let body_bg = Color::new(1.0, 1.0, 1.0, 1.0); // 内容区白色
        let nav_bg = Color::new(0.97, 0.97, 0.97, 1.0); // 导航栏浅灰
        let track_color = Color::new(0.85, 0.85, 0.85, 1.0); // 分隔线颜色

        if is_top {
            // 顶部导航栏
            let nav_rect = Rect::new(bounds.origin.x, bounds.origin.y, w, nav_height);
            ctx.fill_rect(nav_rect, nav_bg, 0.0);

            // 底部轨道线（分隔导航和内容）
            let track_y = bounds.origin.y + nav_height - 1.0;
            let track_rect = Rect::new(bounds.origin.x, track_y, w, 1.0);
            ctx.fill_rect(track_rect, track_color, 0.0);

            // 内容区域
            let body_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y + nav_height,
                w,
                h - nav_height,
            );
            if h > nav_height {
                ctx.fill_rect(body_rect, body_bg, 0.0);
            }
        } else if is_bottom {
            // 底部导航栏
            let body_h = h - nav_height;
            if body_h > 0.0 {
                let body_rect = Rect::new(bounds.origin.x, bounds.origin.y, w, body_h);
                ctx.fill_rect(body_rect, body_bg, 0.0);
            }

            // 顶部轨道线
            let track_y = bounds.origin.y + h - nav_height;
            let track_rect = Rect::new(bounds.origin.x, track_y, w, 1.0);
            ctx.fill_rect(track_rect, track_color, 0.0);

            // 底部导航栏
            let nav_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y + h - nav_height,
                w,
                nav_height,
            );
            ctx.fill_rect(nav_rect, nav_bg, 0.0);
        } else {
            // start/end placement——简化：回退到顶部导航
            let nav_rect = Rect::new(bounds.origin.x, bounds.origin.y, w, nav_height);
            ctx.fill_rect(nav_rect, nav_bg, 0.0);

            let track_y = bounds.origin.y + nav_height - 1.0;
            let track_rect = Rect::new(bounds.origin.x, track_y, w, 1.0);
            ctx.fill_rect(track_rect, track_color, 0.0);

            let body_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y + nav_height,
                w,
                h - nav_height,
            );
            if h > nav_height {
                ctx.fill_rect(body_rect, body_bg, 0.0);
            }
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let _ = state;
        AccessibilityNode::new(
            WidgetId::from_u64(0),
            AccessibilityRole::TabList,
            Rect::ZERO,
        )
        .label("tab group")
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
        let state = WaTabGroupState::new();
        assert_eq!(state.active, "");
        assert_eq!(state.placement, "top");
        assert_eq!(state.activation, "auto");
        assert!(!state.without_scroll_controls);
    }

    #[test]
    fn state_with_active_tab() {
        let state = WaTabGroupState {
            active: "general".into(),
            placement: "top".into(),
            activation: "auto".into(),
            without_scroll_controls: false,
        };
        assert_eq!(state.active, "general");
    }

    #[test]
    fn state_bottom_placement() {
        let state = WaTabGroupState {
            placement: "bottom".into(),
            ..WaTabGroupState::new()
        };
        assert_eq!(state.placement, "bottom");
    }

    #[test]
    fn state_manual_activation() {
        let state = WaTabGroupState {
            activation: "manual".into(),
            ..WaTabGroupState::new()
        };
        assert_eq!(state.activation, "manual");
    }

    #[test]
    fn state_without_scroll_controls() {
        let state = WaTabGroupState {
            without_scroll_controls: true,
            ..WaTabGroupState::new()
        };
        assert!(state.without_scroll_controls);
    }

    #[test]
    fn update_tab_show_noop() {
        let mut state = WaTabGroupState::new();
        let mut ctx = UpdateContext::default();
        WaTabGroup.update(WaTabGroupMessage::TabShow, &mut state, &mut ctx);
        assert_eq!(state.active, ""); // Phase 0 不做切换
    }

    #[test]
    fn update_tab_hide_noop() {
        let mut state = WaTabGroupState {
            active: "tab-1".into(),
            ..WaTabGroupState::new()
        };
        let mut ctx = UpdateContext::default();
        WaTabGroup.update(WaTabGroupMessage::TabHide, &mut state, &mut ctx);
        assert_eq!(state.active, "tab-1"); // Phase 0 不修改
    }

    #[test]
    fn update_scroll_noop() {
        let mut state = WaTabGroupState::new();
        let mut ctx = UpdateContext::default();
        WaTabGroup.update(WaTabGroupMessage::ScrollToStart, &mut state, &mut ctx);
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaTabGroupState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaTabGroup.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_top_placement_produces_ops() {
        let state = WaTabGroupState::new(); // placement = "top"
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaTabGroup.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(!ops.is_empty(), "TabGroup 应产生绘制操作");

        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        // nav bg + track line + body bg = 3 个 FillRect
        assert_eq!(fill_count, 3, "top placement 应有 3 个 FillRect");
    }

    #[test]
    fn paint_bottom_placement_produces_ops() {
        let state = WaTabGroupState {
            placement: "bottom".into(),
            ..WaTabGroupState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaTabGroup.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 3, "bottom placement 应有 3 个 FillRect");
    }

    #[test]
    fn paint_start_placement_falls_back_to_top() {
        let state = WaTabGroupState {
            placement: "start".into(),
            ..WaTabGroupState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaTabGroup.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 3, "start placement 回退到 top 布局");
    }

    #[test]
    fn paint_small_bounds_produces_only_nav() {
        // 高度小于 nav_height 时只绘制导航栏
        let state = WaTabGroupState::new();
        let bounds = Rect::new(0.0, 0.0, 600.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        WaTabGroup.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        // nav bg + track line，body 区域太小（h <= nav_height）不绘制
        assert!(fill_count >= 2, "小 bounds 至少应有 nav + track");
    }

    #[test]
    fn paint_too_small_bounds_returns_early() {
        let state = WaTabGroupState::new();
        let bounds = Rect::new(0.0, 0.0, 3.0, 3.0);
        let mut ctx = PaintContext::new(bounds);
        WaTabGroup.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "过小 bounds 应提前返回");
    }

    #[test]
    fn accessibility_default_state() {
        let state = WaTabGroupState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaTabGroup.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("tab group"));
    }

    #[test]
    fn view_contains_props() {
        let state = WaTabGroupState {
            active: "general".into(),
            placement: "bottom".into(),
            activation: "manual".into(),
            without_scroll_controls: true,
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaTabGroup.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaTabGroup");

        if let Some(PropValue::Str(a)) = view.props.get("active") {
            assert_eq!(a.as_ref(), "general");
        } else {
            panic!("Expected Str prop 'active'");
        }

        if let Some(PropValue::Str(p)) = view.props.get("placement") {
            assert_eq!(p.as_ref(), "bottom");
        } else {
            panic!("Expected Str prop 'placement'");
        }

        if let Some(PropValue::Bool(w)) = view.props.get("without-scroll-controls") {
            assert!(*w);
        } else {
            panic!("Expected Bool prop 'without-scroll-controls'");
        }
    }

    #[test]
    fn view_default_no_active() {
        let state = WaTabGroupState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaTabGroup.view(&state, &ctx);
        if let Some(PropValue::Str(a)) = view.props.get("active") {
            assert!(a.is_empty());
        }
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaTabGroup.name(), "rgui_components::WaTabGroup");
    }
}
