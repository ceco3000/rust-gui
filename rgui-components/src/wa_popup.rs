/// Translated from Web Awesome popup
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// Phase 0 简化:
/// - 无 floating-ui 定位（Phase 0 用 position: absolute + z-index 弹层）
/// - 无 @slot anchor（anchor 由外部组件通过 id/ref 管理）
/// - 无 hover-bridge（鼠标悬停桥接器——Phase 2）
/// - 无 flip/shift/autoSize/sync 等定位策略（Phase 0 仅 placement + distance + skidding）
/// - 无 SUPPORTS_POPOVER 特性检测（rgui 无 Popover API）
///
/// Popup 是弹层家族（Popover/Tooltip/Dropdown）的低层定位引擎。
/// 自身不绘制可视内容（除箭头外），仅作为锚定定位的容器。
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

/// Web Awesome wa-popup 组件状态。
///
/// Popup 是弹层家族的低层定位引擎，自身透明，
/// 仅负责将子内容定位到 anchor 附近。
///
/// Phase 0 简化项：
/// - 所有 floating-ui 定位策略（flip/shift/autoSize/sync）→ 跳过
/// - `boundary`、`flipBoundary`、`shiftBoundary`、`autoSizeBoundary` → 跳过
/// - `hoverBridge` → Phase 2
/// - `arrowPlacement` / `arrowPadding` → 跳过（箭头固定在 placement 对侧边缘）
/// - LocalizeController → 硬编码英文
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaPopupState {
    /// 是否启用定位并显示弹层
    pub active: bool,
    /// 弹出方向：top / top-start / top-end / bottom / bottom-start / bottom-end
    /// / right / right-start / right-end / left / left-start / left-end
    pub placement: String,
    /// 距 anchor 的距离（px）
    pub distance: f64,
    /// 沿 anchor 的偏移（px）
    pub skidding: f64,
    /// 是否显示箭头
    pub arrow: bool,
}

impl Default for WaPopupState {
    fn default() -> Self {
        Self {
            active: false,
            placement: String::new(),
            distance: 0.0,
            skidding: 0.0,
            arrow: false,
        }
    }
}

impl WaPopupState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

/// Popup 事件。
///
/// - `Reposition` — 定位更新（wa-reposition）
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaPopupMessage {
    Reposition,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaPopup;

impl WidgetSpec for WaPopup {
    type State = WaPopupState;
    type Message = WaPopupMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaPopup"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaPopup")
            .prop("active", PropValue::Bool(state.active));

        if !state.placement.is_empty() {
            v = v.prop("placement", PropValue::str(state.placement.as_str()));
        }
        if state.distance > 0.0 {
            v = v.prop(
                "distance",
                PropValue::Float(ordered_float::OrderedFloat(state.distance)),
            );
        }
        if state.skidding > 0.0 {
            v = v.prop(
                "skidding",
                PropValue::Float(ordered_float::OrderedFloat(state.skidding)),
            );
        }
        if state.arrow {
            v = v.prop("arrow", PropValue::Bool(true));
        }

        // 弹层组件：position=absolute + z-index 高值确保浮于内容之上
        if state.active {
            v = v.prop("position", PropValue::Str(std::sync::Arc::from("absolute")));
            v = v.prop("z-index", PropValue::Int(1000));
        }

        v
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaPopupMessage::Reposition => {
                // Phase 0: reposition 事件无实际计算行为
            },
        }
    }

    /// Popup 是容器，尺寸由 Taffy 根据子节点和约束计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if !state.active {
            return;
        }

        // Popup 自身透明，不绘制背景/边框。
        // 子节点通过 Taffy 递归渲染。

        // ── 箭头绘制（可选）──
        if state.arrow {
            let arrow_color = Color::new(1.0, 1.0, 1.0, 1.0); // --arrow-color 默认白
            let arrow_size: f64 = 6.0; // --arrow-size=6px

            // 箭头位置：根据 placement 决定在容器哪一边绘制小三角形
            match state.placement.as_str() {
                "top" | "top-start" | "top-end" => {
                    // 箭头在底部边缘
                    let arrow_x = bounds.origin.x + (bounds.size.width - arrow_size) / 2.0;
                    let arrow_y = bounds.origin.y + bounds.size.height - arrow_size;
                    let arrow_bounds = Rect::new(arrow_x, arrow_y, arrow_size, arrow_size);
                    ctx.fill_rect(arrow_bounds, arrow_color, 0.0);
                },
                "bottom" | "bottom-start" | "bottom-end" | "" => {
                    // 箭头在顶部边缘
                    let arrow_x = bounds.origin.x + (bounds.size.width - arrow_size) / 2.0;
                    let arrow_y = bounds.origin.y;
                    let arrow_bounds = Rect::new(arrow_x, arrow_y, arrow_size, arrow_size);
                    ctx.fill_rect(arrow_bounds, arrow_color, 0.0);
                },
                "right" | "right-start" | "right-end" => {
                    // 箭头在左边边缘
                    let arrow_x = bounds.origin.x;
                    let arrow_y = bounds.origin.y + (bounds.size.height - arrow_size) / 2.0;
                    let arrow_bounds = Rect::new(arrow_x, arrow_y, arrow_size, arrow_size);
                    ctx.fill_rect(arrow_bounds, arrow_color, 0.0);
                },
                "left" | "left-start" | "left-end" => {
                    // 箭头在右边边缘
                    let arrow_x = bounds.origin.x + bounds.size.width - arrow_size;
                    let arrow_y = bounds.origin.y + (bounds.size.height - arrow_size) / 2.0;
                    let arrow_bounds = Rect::new(arrow_x, arrow_y, arrow_size, arrow_size);
                    ctx.fill_rect(arrow_bounds, arrow_color, 0.0);
                },
                _ => {
                    // 默认底部
                    let arrow_x = bounds.origin.x + (bounds.size.width - arrow_size) / 2.0;
                    let arrow_y = bounds.origin.y;
                    let arrow_bounds = Rect::new(arrow_x, arrow_y, arrow_size, arrow_size);
                    ctx.fill_rect(arrow_bounds, arrow_color, 0.0);
                },
            }
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = if state.active {
            AccessibilityRole::Custom("popup")
        } else {
            AccessibilityRole::None
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO).label("popup")
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> ViewContext {
        ViewContext::new(Size::new(800.0, 600.0))
    }

    #[test]
    fn name() {
        assert_eq!(WaPopup.name(), "rgui_components::WaPopup");
    }

    #[test]
    fn default_state() {
        let s = WaPopupState::new();
        assert!(!s.active);
        assert!(s.placement.is_empty());
        assert_eq!(s.distance, 0.0);
        assert_eq!(s.skidding, 0.0);
        assert!(!s.arrow);
    }

    #[test]
    fn state_active() {
        let s = WaPopupState {
            active: true,
            ..WaPopupState::new()
        };
        assert!(s.active);
    }

    #[test]
    fn state_with_placement() {
        let s = WaPopupState {
            placement: "bottom-start".into(),
            ..WaPopupState::new()
        };
        assert_eq!(s.placement, "bottom-start");
    }

    #[test]
    fn state_with_distance() {
        let s = WaPopupState {
            distance: 8.0,
            ..WaPopupState::new()
        };
        assert_eq!(s.distance, 8.0);
    }

    #[test]
    fn state_with_skidding() {
        let s = WaPopupState {
            skidding: 4.0,
            ..WaPopupState::new()
        };
        assert_eq!(s.skidding, 4.0);
    }

    #[test]
    fn state_with_arrow() {
        let s = WaPopupState {
            arrow: true,
            ..WaPopupState::new()
        };
        assert!(s.arrow);
    }

    #[test]
    fn view_contains_core_props() {
        let s = WaPopupState::new();
        let v = WaPopup.view(&s, &make_ctx());
        assert!(v.props.contains_key("active"));
    }

    #[test]
    fn view_active_prop() {
        let s = WaPopupState {
            active: true,
            ..WaPopupState::new()
        };
        let v = WaPopup.view(&s, &make_ctx());
        let val = v.props.get("active").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for active"),
        }
    }

    #[test]
    fn view_placement_prop() {
        let s = WaPopupState {
            placement: "bottom-start".into(),
            ..WaPopupState::new()
        };
        let v = WaPopup.view(&s, &make_ctx());
        let val = v.props.get("placement").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "bottom-start"),
            _ => panic!("expected Str prop for placement"),
        }
    }

    #[test]
    fn view_distance_prop() {
        let s = WaPopupState {
            distance: 8.0,
            ..WaPopupState::new()
        };
        let v = WaPopup.view(&s, &make_ctx());
        let val = v.props.get("distance").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 8.0),
            _ => panic!("expected Float prop for distance"),
        }
    }

    #[test]
    fn view_skidding_prop() {
        let s = WaPopupState {
            skidding: 4.0,
            ..WaPopupState::new()
        };
        let v = WaPopup.view(&s, &make_ctx());
        let val = v.props.get("skidding").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 4.0),
            _ => panic!("expected Float prop for skidding"),
        }
    }

    #[test]
    fn view_arrow_prop() {
        let s = WaPopupState {
            arrow: true,
            ..WaPopupState::new()
        };
        let v = WaPopup.view(&s, &make_ctx());
        let val = v.props.get("arrow").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for arrow"),
        }
    }

    #[test]
    fn view_active_adds_position_absolute() {
        let s = WaPopupState {
            active: true,
            ..WaPopupState::new()
        };
        let v = WaPopup.view(&s, &make_ctx());
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "absolute"),
            _ => panic!("expected Str prop for position"),
        }
    }

    #[test]
    fn view_active_adds_z_index() {
        let s = WaPopupState {
            active: true,
            ..WaPopupState::new()
        };
        let v = WaPopup.view(&s, &make_ctx());
        let val = v.props.get("z-index").unwrap();
        match val {
            PropValue::Int(i) => assert_eq!(*i, 1000),
            _ => panic!("expected Int prop for z-index"),
        }
    }

    #[test]
    fn view_inactive_no_position_z_index() {
        let s = WaPopupState::new(); // active = false
        let v = WaPopup.view(&s, &make_ctx());
        assert!(!v.props.contains_key("position"));
        assert!(!v.props.contains_key("z-index"));
    }

    #[test]
    fn update_reposition_noop() {
        let mut s = WaPopupState {
            active: true,
            ..WaPopupState::new()
        };
        WaPopup.update(
            WaPopupMessage::Reposition,
            &mut s,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaPopupState::new();
        let size = WaPopup.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Popup 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_inactive_produces_no_ops() {
        let s = WaPopupState::new(); // active = false
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopup.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "关闭的 Popup 不应绘制任何内容");
    }

    #[test]
    fn paint_active_no_arrow_produces_no_ops() {
        let s = WaPopupState {
            active: true,
            ..WaPopupState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopup.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "无箭头的 Popup 自身不绘制");
    }

    #[test]
    fn paint_active_with_arrow_produces_ops() {
        let s = WaPopupState {
            active: true,
            arrow: true,
            placement: "bottom".into(),
            ..WaPopupState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopup.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "带箭头的 Popup 应绘制箭头");
    }

    #[test]
    fn paint_active_arrow_top_placement() {
        let s = WaPopupState {
            active: true,
            arrow: true,
            placement: "top".into(),
            ..WaPopupState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopup.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "top placement 箭头应绘制");
    }

    #[test]
    fn paint_active_arrow_left_placement() {
        let s = WaPopupState {
            active: true,
            arrow: true,
            placement: "left".into(),
            ..WaPopupState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopup.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "left placement 箭头应绘制");
    }

    #[test]
    fn paint_active_arrow_right_placement() {
        let s = WaPopupState {
            active: true,
            arrow: true,
            placement: "right".into(),
            ..WaPopupState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopup.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "right placement 箭头应绘制");
    }

    #[test]
    fn accessibility_active() {
        let s = WaPopupState {
            active: true,
            ..WaPopupState::new()
        };
        let node = WaPopup.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("popup"));
    }

    #[test]
    fn accessibility_inactive() {
        let s = WaPopupState::new();
        let node = WaPopup.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("popup"));
    }
}
