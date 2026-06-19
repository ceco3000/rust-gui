/// Translated from Web Awesome popover
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// Phase 0 简化:
/// - 无 wa-popup 依赖（直接用 position: absolute + z-index 弹层）
/// - 无 floating-ui 定位（Phase 0 用固定位置）
/// - 无动画（show/hide CSS 类 + animateWithClass）
/// - 无 <dialog> HTML 元素语义
/// - 无 dismissible stack（WTI03 通过框架 handle_click 发送 Close）
/// - 无键盘 Escape 处理（由框架事件系统后续支持）
/// - 无 anchor 绑定（for 属性查找/click 事件—Phase 0 由外部组件管理）
///
/// Popover 在 anchor 附近显示浮动面板，包含上下文内容和交互元素。
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

/// Web Awesome wa-popover 组件状态。
///
/// Popover 在 anchor 附近显示浮动面板，包含上下文内容和交互元素。
/// 适用于 rich tooltip、菜单等按需显示的弹层。
///
/// Phase 0 简化项：
/// - `for`（anchor 元素 ID）→ 跳过（外部组件管理）
/// - 动画状态 → Phase 2
/// - LocalizeController → 硬编码英文
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaPopoverState {
    /// 是否显示弹层
    pub open: bool,
    /// 弹出方向：top / top-start / top-end / bottom / bottom-start / bottom-end
    /// / right / right-start / right-end / left / left-start / left-end
    pub placement: String,
    /// 距 anchor 的距离（px），默认 8
    pub distance: f64,
    /// 沿 anchor 的偏移（px）
    pub skidding: f64,
    /// 是否隐藏箭头
    pub without_arrow: bool,
}

impl Default for WaPopoverState {
    fn default() -> Self {
        Self {
            open: false,
            placement: String::new(),
            distance: 8.0,
            skidding: 0.0,
            without_arrow: false,
        }
    }
}

impl WaPopoverState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

/// Popover 事件。
///
/// - `Show` — 弹层即将显示（wa-show，可取消）
/// - `AfterShow` — 弹层已显示
/// - `Hide` — 弹层即将隐藏（wa-hide，可取消）
/// - `AfterHide` — 弹层已隐藏
/// - `Close` — WTI03 框架发送的关闭指令（点击外部）
///
/// Phase 0：除 Close 外所有事件无实际行为。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaPopoverMessage {
    Show,
    AfterShow,
    Hide,
    AfterHide,
    Close,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaPopover;

impl WidgetSpec for WaPopover {
    type State = WaPopoverState;
    type Message = WaPopoverMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaPopover"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaPopover")
            .prop("open", PropValue::Bool(state.open));

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
        if state.without_arrow {
            v = v.prop("without-arrow", PropValue::Bool(true));
        }

        // 弹层组件：position=absolute + z-index 高值确保浮于内容之上
        if state.open {
            v = v.prop(
                "position",
                PropValue::Str(std::sync::Arc::from("absolute")),
            );
            v = v.prop("z-index", PropValue::Int(1000));
        }

        v
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaPopoverMessage::Close => {
                state.open = false;
            }
            // Phase 0: 其他事件无实际行为
            WaPopoverMessage::Show => {}
            WaPopoverMessage::AfterShow => {}
            WaPopoverMessage::Hide => {}
            WaPopoverMessage::AfterHide => {}
        }
    }

    /// Popover 是容器，尺寸由 Taffy 根据子节点和约束计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if !state.open {
            return;
        }

        let border_radius: f32 = 10.0; // --wa-border-radius-l
        let panel_bg = Color::new(1.0, 1.0, 1.0, 1.0); // --wa-color-surface-raised
        let panel_border = Color::new(0.85, 0.85, 0.85, 1.0);
        let shadow_color = Color::new(0.0, 0.0, 0.0, 0.10);
        let arrow_color = Color::new(1.0, 1.0, 1.0, 1.0);

        // ── 计算面板尺寸与位置 ──
        // --max-width = 25rem (400px)
        let max_panel_w: f64 = 400.0;
        let panel_w: f64 = max_panel_w.min(bounds.size.width - 16.0).max(100.0);
        let panel_h: f64 = (bounds.size.height * 0.5).min(300.0).max(40.0);

        // Phase 0: 根据 placement 确定面板位置
        let (panel_x, panel_y) = match state.placement.as_str() {
            "top" | "top-start" => {
                let x = bounds.origin.x + state.skidding;
                let y = bounds.origin.y - panel_h - state.distance;
                (x, y)
            }
            "top-end" => {
                let x = bounds.origin.x + bounds.size.width - panel_w - state.skidding;
                let y = bounds.origin.y - panel_h - state.distance;
                (x, y)
            }
            "bottom" | "bottom-start" | "" => {
                let x = bounds.origin.x + state.skidding;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            }
            "bottom-end" => {
                let x = bounds.origin.x + bounds.size.width - panel_w - state.skidding;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            }
            "right" | "right-start" => {
                let x = bounds.origin.x + bounds.size.width + state.distance;
                let y = bounds.origin.y + state.skidding;
                (x, y)
            }
            "right-end" => {
                let x = bounds.origin.x + bounds.size.width + state.distance;
                let y = bounds.origin.y + bounds.size.height - panel_h - state.skidding;
                (x, y)
            }
            "left" | "left-start" => {
                let x = bounds.origin.x - panel_w - state.distance;
                let y = bounds.origin.y + state.skidding;
                (x, y)
            }
            "left-end" => {
                let x = bounds.origin.x - panel_w - state.distance;
                let y = bounds.origin.y + bounds.size.height - panel_h - state.skidding;
                (x, y)
            }
            _ => {
                let x = bounds.origin.x + state.skidding;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            }
        };

        let panel_bounds = Rect::new(panel_x, panel_y, panel_w, panel_h);

        // ── 1. 阴影（偏移绘制近似 box-shadow）──
        let shadow_offset: f64 = 3.0;
        let shadow_bounds = Rect::new(
            panel_x + shadow_offset,
            panel_y + shadow_offset,
            panel_w,
            panel_h,
        );
        ctx.fill_rect(shadow_bounds, shadow_color, border_radius);

        // ── 2. 面板背景 ──
        ctx.fill_rect(panel_bounds, panel_bg, border_radius);

        // ── 3. 边框 ──
        let border_w: f64 = 1.0;
        ctx.fill_rect(
            Rect::new(
                panel_x + border_w,
                panel_y + border_w,
                panel_w - 2.0 * border_w,
                panel_h - 2.0 * border_w,
            ),
            panel_border,
            border_radius - 1.0,
        );
        ctx.fill_rect(
            Rect::new(
                panel_x + border_w,
                panel_y + border_w,
                panel_w - 2.0 * border_w,
                panel_h - 2.0 * border_w,
            ),
            panel_bg,
            (border_radius - 1.0).max(0.0),
        );

        // ── 4. 箭头（若不隐藏）──
        if !state.without_arrow {
            let arrow_size: f64 = 8.0;
            match state.placement.as_str() {
                "top" | "top-start" | "top-end" => {
                    let ax = panel_x + (panel_w - arrow_size) / 2.0;
                    let ay = panel_y + panel_h;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                "bottom" | "bottom-start" | "bottom-end" | "" => {
                    let ax = panel_x + (panel_w - arrow_size) / 2.0;
                    let ay = panel_y - arrow_size;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                "right" | "right-start" | "right-end" => {
                    let ax = panel_x - arrow_size;
                    let ay = panel_y + (panel_h - arrow_size) / 2.0;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                "left" | "left-start" | "left-end" => {
                    let ax = panel_x + panel_w;
                    let ay = panel_y + (panel_h - arrow_size) / 2.0;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                _ => {
                    let ax = panel_x + (panel_w - arrow_size) / 2.0;
                    let ay = panel_y - arrow_size;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
            }
        }

        // ── 5. Body 内容区域（透明，由子节点渲染）──
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = if state.open {
            AccessibilityRole::Dialog
        } else {
            AccessibilityRole::None
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO)
            .label("popover")
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
        assert_eq!(WaPopover.name(), "rgui_components::WaPopover");
    }

    #[test]
    fn default_state() {
        let s = WaPopoverState::new();
        assert!(!s.open);
        assert!(s.placement.is_empty());
        assert_eq!(s.distance, 8.0);
        assert_eq!(s.skidding, 0.0);
        assert!(!s.without_arrow);
    }

    #[test]
    fn state_open() {
        let s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        assert!(s.open);
    }

    #[test]
    fn state_with_placement() {
        let s = WaPopoverState {
            placement: "bottom-start".into(),
            ..WaPopoverState::new()
        };
        assert_eq!(s.placement, "bottom-start");
    }

    #[test]
    fn state_with_distance() {
        let s = WaPopoverState {
            distance: 12.0,
            ..WaPopoverState::new()
        };
        assert_eq!(s.distance, 12.0);
    }

    #[test]
    fn state_with_skidding() {
        let s = WaPopoverState {
            skidding: 8.0,
            ..WaPopoverState::new()
        };
        assert_eq!(s.skidding, 8.0);
    }

    #[test]
    fn state_without_arrow() {
        let s = WaPopoverState {
            without_arrow: true,
            ..WaPopoverState::new()
        };
        assert!(s.without_arrow);
    }

    #[test]
    fn view_contains_core_props() {
        let s = WaPopoverState::new();
        let v = WaPopover.view(&s, &make_ctx());
        assert!(v.props.contains_key("open"));
    }

    #[test]
    fn view_open_prop() {
        let s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        let v = WaPopover.view(&s, &make_ctx());
        let val = v.props.get("open").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for open"),
        }
    }

    #[test]
    fn view_placement_prop() {
        let s = WaPopoverState {
            placement: "bottom-start".into(),
            ..WaPopoverState::new()
        };
        let v = WaPopover.view(&s, &make_ctx());
        let val = v.props.get("placement").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "bottom-start"),
            _ => panic!("expected Str prop for placement"),
        }
    }

    #[test]
    fn view_distance_prop() {
        let s = WaPopoverState {
            distance: 12.0,
            ..WaPopoverState::new()
        };
        let v = WaPopover.view(&s, &make_ctx());
        let val = v.props.get("distance").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 12.0),
            _ => panic!("expected Float prop for distance"),
        }
    }

    #[test]
    fn view_skidding_prop() {
        let s = WaPopoverState {
            skidding: 8.0,
            ..WaPopoverState::new()
        };
        let v = WaPopover.view(&s, &make_ctx());
        let val = v.props.get("skidding").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 8.0),
            _ => panic!("expected Float prop for skidding"),
        }
    }

    #[test]
    fn view_without_arrow_prop() {
        let s = WaPopoverState {
            without_arrow: true,
            ..WaPopoverState::new()
        };
        let v = WaPopover.view(&s, &make_ctx());
        let val = v.props.get("without-arrow").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for without-arrow"),
        }
    }

    #[test]
    fn view_open_adds_position_absolute() {
        let s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        let v = WaPopover.view(&s, &make_ctx());
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "absolute"),
            _ => panic!("expected Str prop for position"),
        }
    }

    #[test]
    fn view_open_adds_z_index() {
        let s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        let v = WaPopover.view(&s, &make_ctx());
        let val = v.props.get("z-index").unwrap();
        match val {
            PropValue::Int(i) => assert_eq!(*i, 1000),
            _ => panic!("expected Int prop for z-index"),
        }
    }

    #[test]
    fn view_closed_no_position_z_index() {
        let s = WaPopoverState::new(); // open = false
        let v = WaPopover.view(&s, &make_ctx());
        assert!(!v.props.contains_key("position"));
        assert!(!v.props.contains_key("z-index"));
    }

    #[test]
    fn update_close_sets_open_false() {
        let mut s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        WaPopover.update(
            WaPopoverMessage::Close,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.open, "Close 应将 open 设为 false");
    }

    #[test]
    fn update_show_noop() {
        let mut s = WaPopoverState::new();
        WaPopover.update(
            WaPopoverMessage::Show,
            &mut s,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_after_show_noop() {
        let mut s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        WaPopover.update(
            WaPopoverMessage::AfterShow,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open);
    }

    #[test]
    fn update_hide_noop() {
        let mut s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        WaPopover.update(
            WaPopoverMessage::Hide,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open, "Phase 0: Hide 事件不自动关闭");
    }

    #[test]
    fn update_after_hide_noop() {
        let mut s = WaPopoverState::new();
        WaPopover.update(
            WaPopoverMessage::AfterHide,
            &mut s,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaPopoverState::new();
        let size = WaPopover.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Popover 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_closed_produces_no_ops() {
        let s = WaPopoverState::new(); // open = false
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopover.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "关闭的 Popover 不应绘制任何内容");
    }

    #[test]
    fn paint_open_produces_ops() {
        let s = WaPopoverState {
            open: true,
            placement: "bottom".into(),
            ..WaPopoverState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopover.paint(&s, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 4,
            "打开的 Popover 应产生多个绘制操作（阴影+背景+边框+箭头），实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_open_without_arrow_fewer_ops() {
        let s = WaPopoverState {
            open: true,
            without_arrow: true,
            ..WaPopoverState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopover.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3, "without-arrow Popover 仍有基础绘制");
    }

    #[test]
    fn paint_open_top_placement() {
        let s = WaPopoverState {
            open: true,
            placement: "top".into(),
            ..WaPopoverState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopover.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "top placement Popover 应绘制");
    }

    #[test]
    fn paint_open_left_placement() {
        let s = WaPopoverState {
            open: true,
            placement: "left".into(),
            ..WaPopoverState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopover.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "left placement Popover 应绘制");
    }

    #[test]
    fn paint_open_right_placement() {
        let s = WaPopoverState {
            open: true,
            placement: "right".into(),
            ..WaPopoverState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaPopover.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "right placement Popover 应绘制");
    }

    #[test]
    fn accessibility_open() {
        let s = WaPopoverState {
            open: true,
            ..WaPopoverState::new()
        };
        let node = WaPopover.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("popover"));
    }
}
