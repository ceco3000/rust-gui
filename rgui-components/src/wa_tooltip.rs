/// Translated from Web Awesome tooltip
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// Phase 0 简化:
/// - 无 wa-popup 依赖（直接用 position: absolute + z-index 弹层）
/// - 无 floating-ui 定位（Phase 0 用固定位置）
/// - 无动画（show/hide CSS 类 + animateWithClass）
/// - 无 dismissible stack（WTI03 通过框架 handle_click 发送 Close）
/// - 无键盘 Escape 处理（由框架事件系统后续支持）
/// - 无 hover/focus/blur 事件绑定（Phase 0 仅程序化 show/hide）
/// - 无 showDelay/hideDelay 延迟逻辑（Phase 0 即时切换）
/// - 无 aria-labelledby 管理
///
/// Tooltip 在 hover/focus 时显示简短的上下文信息。
/// 不应包含交互元素。
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

/// Web Awesome wa-tooltip 组件状态。
///
/// Tooltip 在 hover/focus 时显示简短的上下文信息。
/// 不应包含交互元素——仅用于信息展示。
///
/// Phase 0 简化项：
/// - `showDelay` / `hideDelay` → 跳过（即时切换）
/// - `trigger` → 跳过（Phase 0 仅程序化控制）
/// - `for` → 跳过（外部组件管理 anchor）
/// - `disabled` → 保留（Phase 0：阻止显示）
/// - 动画状态 → Phase 2
/// - LocalizeController → 硬编码英文
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaTooltipState {
    /// 是否显示 tooltip
    pub open: bool,
    /// 禁用 tooltip（不显示）
    pub disabled: bool,
    /// 弹出方向：top / top-start / top-end / bottom / bottom-start / bottom-end
    /// / right / right-start / right-end / left / left-start / left-end
    pub placement: String,
    /// 距 anchor 的距离（px），默认 8
    pub distance: f64,
    /// 沿 anchor 的偏移（px）
    pub skidding: f64,
    /// 显示延迟（ms），Phase 0：不实现延迟逻辑
    pub show_delay: f64,
    /// 隐藏延迟（ms），Phase 0：不实现延迟逻辑
    pub hide_delay: f64,
    /// 触发方式（空格分隔），如 "hover focus"
    pub trigger: String,
    /// 是否隐藏箭头
    pub without_arrow: bool,
}

impl Default for WaTooltipState {
    fn default() -> Self {
        Self {
            open: false,
            disabled: false,
            placement: String::new(),
            distance: 8.0,
            skidding: 0.0,
            show_delay: 150.0,
            hide_delay: 0.0,
            trigger: String::new(),
            without_arrow: false,
        }
    }
}

impl WaTooltipState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

/// Tooltip 事件。
///
/// - `Show` — tooltip 即将显示（wa-show，可取消）
/// - `AfterShow` — tooltip 已显示
/// - `Hide` — tooltip 即将隐藏（wa-hide，可取消）
/// - `AfterHide` — tooltip 已隐藏
///
/// Phase 0：所有事件无实际行为（仅切换 open 状态）。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTooltipMessage {
    Show,
    AfterShow,
    Hide,
    AfterHide,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaTooltip;

impl WidgetSpec for WaTooltip {
    type State = WaTooltipState;
    type Message = WaTooltipMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTooltip"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaTooltip")
            .prop("open", PropValue::Bool(state.open))
            .prop("disabled", PropValue::Bool(state.disabled));

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
        if state.show_delay > 0.0 {
            v = v.prop(
                "show-delay",
                PropValue::Float(ordered_float::OrderedFloat(state.show_delay)),
            );
        }
        if state.hide_delay > 0.0 {
            v = v.prop(
                "hide-delay",
                PropValue::Float(ordered_float::OrderedFloat(state.hide_delay)),
            );
        }
        if !state.trigger.is_empty() {
            v = v.prop("trigger", PropValue::str(state.trigger.as_str()));
        }
        if state.without_arrow {
            v = v.prop("without-arrow", PropValue::Bool(true));
        }

        // 弹层组件：position=absolute + z-index 高值确保浮于内容之上
        if state.open && !state.disabled {
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
            WaTooltipMessage::Show => {
                if !state.disabled {
                    state.open = true;
                }
            }
            WaTooltipMessage::Hide => {
                state.open = false;
            }
            // Phase 0: After 事件无额外行为
            WaTooltipMessage::AfterShow => {}
            WaTooltipMessage::AfterHide => {}
        }
    }

    /// Tooltip 是容器，尺寸由 Taffy 根据子节点和约束计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if !state.open || state.disabled {
            return;
        }

        let border_radius: f32 = 6.0; // --wa-border-radius-s
        let bg = Color::new(0.13, 0.13, 0.13, 0.95); // --wa-color-neutral-900
        let text_color = Color::new(1.0, 1.0, 1.0, 1.0);
        let arrow_color = Color::new(0.13, 0.13, 0.13, 0.95);

        // ── 计算 tooltip 尺寸与位置 ──
        // --max-width = unspecified, default to reasonable
        let max_w: f64 = 300.0;
        let tooltip_w: f64 = max_w.min(bounds.size.width - 16.0).max(40.0);
        let tooltip_h: f64 = 28.0; // 单行 tooltip 高度

        // Phase 0: 根据 placement 确定位置
        let (tip_x, tip_y) = match state.placement.as_str() {
            "top" | "top-start" => {
                let x = bounds.origin.x + state.skidding;
                let y = bounds.origin.y - tooltip_h - state.distance;
                (x, y)
            }
            "top-end" => {
                let x = bounds.origin.x + bounds.size.width - tooltip_w - state.skidding;
                let y = bounds.origin.y - tooltip_h - state.distance;
                (x, y)
            }
            "bottom" | "bottom-start" | "" => {
                let x = bounds.origin.x + state.skidding;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            }
            "bottom-end" => {
                let x = bounds.origin.x + bounds.size.width - tooltip_w - state.skidding;
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
                let y = bounds.origin.y + bounds.size.height - tooltip_h - state.skidding;
                (x, y)
            }
            "left" | "left-start" => {
                let x = bounds.origin.x - tooltip_w - state.distance;
                let y = bounds.origin.y + state.skidding;
                (x, y)
            }
            "left-end" => {
                let x = bounds.origin.x - tooltip_w - state.distance;
                let y = bounds.origin.y + bounds.size.height - tooltip_h - state.skidding;
                (x, y)
            }
            _ => {
                let x = bounds.origin.x + state.skidding;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            }
        };

        let tip_bounds = Rect::new(tip_x, tip_y, tooltip_w, tooltip_h);

        // ── 1. tooltip 背景（暗色圆角矩形）──
        ctx.fill_rect(tip_bounds, bg, border_radius);

        // ── 2. 内容文本居中 ──
        let font_size: f32 = 12.0;
        ctx.draw_text("...", tip_bounds, text_color, font_size);

        // ── 3. 箭头（若不隐藏）──
        if !state.without_arrow {
            let arrow_size: f64 = 6.0;
            match state.placement.as_str() {
                "top" | "top-start" | "top-end" => {
                    let ax = tip_x + (tooltip_w - arrow_size) / 2.0;
                    let ay = tip_y + tooltip_h;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                "bottom" | "bottom-start" | "bottom-end" | "" => {
                    let ax = tip_x + (tooltip_w - arrow_size) / 2.0;
                    let ay = tip_y - arrow_size;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                "right" | "right-start" | "right-end" => {
                    let ax = tip_x - arrow_size;
                    let ay = tip_y + (tooltip_h - arrow_size) / 2.0;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                "left" | "left-start" | "left-end" => {
                    let ax = tip_x + tooltip_w;
                    let ay = tip_y + (tooltip_h - arrow_size) / 2.0;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
                _ => {
                    let ax = tip_x + (tooltip_w - arrow_size) / 2.0;
                    let ay = tip_y - arrow_size;
                    ctx.fill_rect(
                        Rect::new(ax, ay, arrow_size, arrow_size),
                        arrow_color,
                        0.0,
                    );
                }
            }
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = if state.open && !state.disabled {
            AccessibilityRole::Tooltip
        } else {
            AccessibilityRole::None
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO)
            .label("tooltip")
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
        assert_eq!(WaTooltip.name(), "rgui_components::WaTooltip");
    }

    #[test]
    fn default_state() {
        let s = WaTooltipState::new();
        assert!(!s.open);
        assert!(!s.disabled);
        assert!(s.placement.is_empty());
        assert_eq!(s.distance, 8.0);
        assert_eq!(s.skidding, 0.0);
        assert_eq!(s.show_delay, 150.0);
        assert_eq!(s.hide_delay, 0.0);
        assert!(s.trigger.is_empty());
        assert!(!s.without_arrow);
    }

    #[test]
    fn state_open() {
        let s = WaTooltipState {
            open: true,
            ..WaTooltipState::new()
        };
        assert!(s.open);
    }

    #[test]
    fn state_disabled() {
        let s = WaTooltipState {
            disabled: true,
            ..WaTooltipState::new()
        };
        assert!(s.disabled);
    }

    #[test]
    fn state_with_placement() {
        let s = WaTooltipState {
            placement: "top".into(),
            ..WaTooltipState::new()
        };
        assert_eq!(s.placement, "top");
    }

    #[test]
    fn state_with_distance() {
        let s = WaTooltipState {
            distance: 12.0,
            ..WaTooltipState::new()
        };
        assert_eq!(s.distance, 12.0);
    }

    #[test]
    fn state_with_skidding() {
        let s = WaTooltipState {
            skidding: 8.0,
            ..WaTooltipState::new()
        };
        assert_eq!(s.skidding, 8.0);
    }

    #[test]
    fn state_with_trigger() {
        let s = WaTooltipState {
            trigger: "hover focus".into(),
            ..WaTooltipState::new()
        };
        assert_eq!(s.trigger, "hover focus");
    }

    #[test]
    fn state_without_arrow() {
        let s = WaTooltipState {
            without_arrow: true,
            ..WaTooltipState::new()
        };
        assert!(s.without_arrow);
    }

    #[test]
    fn view_contains_core_props() {
        let s = WaTooltipState::new();
        let v = WaTooltip.view(&s, &make_ctx());
        assert!(v.props.contains_key("open"));
        assert!(v.props.contains_key("disabled"));
    }

    #[test]
    fn view_open_prop() {
        let s = WaTooltipState {
            open: true,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("open").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for open"),
        }
    }

    #[test]
    fn view_disabled_prop() {
        let s = WaTooltipState {
            disabled: true,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("disabled").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for disabled"),
        }
    }

    #[test]
    fn view_placement_prop() {
        let s = WaTooltipState {
            placement: "top".into(),
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("placement").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "top"),
            _ => panic!("expected Str prop for placement"),
        }
    }

    #[test]
    fn view_distance_prop() {
        let s = WaTooltipState {
            distance: 12.0,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("distance").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 12.0),
            _ => panic!("expected Float prop for distance"),
        }
    }

    #[test]
    fn view_skidding_prop() {
        let s = WaTooltipState {
            skidding: 8.0,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("skidding").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 8.0),
            _ => panic!("expected Float prop for skidding"),
        }
    }

    #[test]
    fn view_show_delay_prop() {
        let s = WaTooltipState {
            show_delay: 200.0,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("show-delay").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 200.0),
            _ => panic!("expected Float prop for show-delay"),
        }
    }

    #[test]
    fn view_trigger_prop() {
        let s = WaTooltipState {
            trigger: "hover".into(),
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("trigger").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "hover"),
            _ => panic!("expected Str prop for trigger"),
        }
    }

    #[test]
    fn view_without_arrow_prop() {
        let s = WaTooltipState {
            without_arrow: true,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("without-arrow").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for without-arrow"),
        }
    }

    #[test]
    fn view_open_disabled_no_position_z_index() {
        let s = WaTooltipState {
            open: true,
            disabled: true,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        assert!(
            !v.props.contains_key("position"),
            "disabled 时不应有 position"
        );
        assert!(!v.props.contains_key("z-index"), "disabled 时不应有 z-index");
    }

    #[test]
    fn view_open_adds_position_absolute() {
        let s = WaTooltipState {
            open: true,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "absolute"),
            _ => panic!("expected Str prop for position"),
        }
    }

    #[test]
    fn view_open_adds_z_index() {
        let s = WaTooltipState {
            open: true,
            ..WaTooltipState::new()
        };
        let v = WaTooltip.view(&s, &make_ctx());
        let val = v.props.get("z-index").unwrap();
        match val {
            PropValue::Int(i) => assert_eq!(*i, 1000),
            _ => panic!("expected Int prop for z-index"),
        }
    }

    #[test]
    fn view_closed_no_position_z_index() {
        let s = WaTooltipState::new(); // open = false
        let v = WaTooltip.view(&s, &make_ctx());
        assert!(!v.props.contains_key("position"));
        assert!(!v.props.contains_key("z-index"));
    }

    #[test]
    fn update_show_sets_open() {
        let mut s = WaTooltipState::new();
        WaTooltip.update(
            WaTooltipMessage::Show,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open, "Show 应将 open 设为 true");
    }

    #[test]
    fn update_show_disabled_noop() {
        let mut s = WaTooltipState {
            disabled: true,
            ..WaTooltipState::new()
        };
        WaTooltip.update(
            WaTooltipMessage::Show,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.open, "disabled 时 Show 不应打开");
    }

    #[test]
    fn update_hide_sets_open_false() {
        let mut s = WaTooltipState {
            open: true,
            ..WaTooltipState::new()
        };
        WaTooltip.update(
            WaTooltipMessage::Hide,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.open, "Hide 应将 open 设为 false");
    }

    #[test]
    fn update_after_show_noop() {
        let mut s = WaTooltipState {
            open: true,
            ..WaTooltipState::new()
        };
        WaTooltip.update(
            WaTooltipMessage::AfterShow,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open);
    }

    #[test]
    fn update_after_hide_noop() {
        let mut s = WaTooltipState::new();
        WaTooltip.update(
            WaTooltipMessage::AfterHide,
            &mut s,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaTooltipState::new();
        let size = WaTooltip.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Tooltip 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_closed_produces_no_ops() {
        let s = WaTooltipState::new(); // open = false
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaTooltip.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "关闭的 Tooltip 不应绘制任何内容");
    }

    #[test]
    fn paint_disabled_produces_no_ops() {
        let s = WaTooltipState {
            open: true,
            disabled: true,
            ..WaTooltipState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaTooltip.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "disabled Tooltip 不应绘制");
    }

    #[test]
    fn paint_open_produces_ops() {
        let s = WaTooltipState {
            open: true,
            placement: "bottom".into(),
            ..WaTooltipState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaTooltip.paint(&s, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 2,
            "打开的 Tooltip 应产生多个绘制操作（背景+文本+箭头），实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_open_without_arrow_fewer_ops() {
        let s = WaTooltipState {
            open: true,
            without_arrow: true,
            ..WaTooltipState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaTooltip.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2, "without-arrow Tooltip 仍有背景和文本");
    }

    #[test]
    fn paint_open_top_placement() {
        let s = WaTooltipState {
            open: true,
            placement: "top".into(),
            ..WaTooltipState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaTooltip.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "top placement Tooltip 应绘制");
    }

    #[test]
    fn paint_open_left_placement() {
        let s = WaTooltipState {
            open: true,
            placement: "left".into(),
            ..WaTooltipState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaTooltip.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "left placement Tooltip 应绘制");
    }

    #[test]
    fn paint_open_right_placement() {
        let s = WaTooltipState {
            open: true,
            placement: "right".into(),
            ..WaTooltipState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaTooltip.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "right placement Tooltip 应绘制");
    }

    #[test]
    fn accessibility_open() {
        let s = WaTooltipState {
            open: true,
            ..WaTooltipState::new()
        };
        let node = WaTooltip.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("tooltip"));
    }

    #[test]
    fn accessibility_disabled() {
        let s = WaTooltipState {
            open: true,
            disabled: true,
            ..WaTooltipState::new()
        };
        let node = WaTooltip.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("tooltip"));
    }
}
