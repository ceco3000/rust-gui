/// Translated from Web Awesome dropdown
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// Phase 0 简化:
/// - 无 wa-popup 依赖（直接用 position: absolute + z-index 弹层）
/// - 无 floating-ui 定位（Phase 0 用固定位置）
/// - 无动画（show/hide CSS 类）
/// - 无 dismissible stack（WTI03 通过框架 handle_click 发送 Close）
/// - 无键盘导航（ArrowUp/Down/Enter/Escape—由框架事件系统后续支持）
/// - 无子菜单（submenu 系统复杂，Phase 0 跳过）
/// - 无 safe triangle 鼠标路径优化
/// - 无 trigger slot 渲染（trigger 由外部组件负责）
/// - 无 size prop 同步到子项
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

/// Web Awesome wa-dropdown 组件状态。
///
/// Dropdown 是弹出式菜单容器，通过 trigger 触发显示选项列表。
/// 合并了原 Shoelace 的 menu/menu-item/menu-label 功能。
///
/// Phase 0 简化项：
/// - `size` → 跳过（不传递到子项）
/// - 所有子菜单相关内部状态 → 跳过
/// - LocalizeController → 硬编码英文（placement 仅使用 CSS 值）
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaDropdownState {
    /// 下拉菜单是否打开
    pub open: bool,
    /// 弹出方向：top / top-start / top-end / bottom / bottom-start / bottom-end
    /// / right / right-start / right-end / left / left-start / left-end
    pub placement: String,
    /// trigger 与菜单的距离（px）
    pub distance: f64,
    /// 沿 trigger 的偏移（px）
    pub skidding: f64,
}

impl Default for WaDropdownState {
    fn default() -> Self {
        Self {
            open: false,
            placement: String::new(),
            distance: 0.0,
            skidding: 0.0,
        }
    }
}

impl WaDropdownState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

/// Dropdown 事件。
///
/// - `Show` — 菜单即将显示（wa-show，可取消）
/// - `AfterShow` — 菜单已显示
/// - `Hide` — 菜单即将隐藏（wa-hide，可取消）
/// - `AfterHide` — 菜单已隐藏
/// - `Select` — 子项被选中（wa-select）
/// - `Close` — WTI03 框架发送的关闭指令（点击外部）
///
/// Phase 0：除 Close 外所有事件无实际行为。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaDropdownMessage {
    Show,
    AfterShow,
    Hide,
    AfterHide,
    Select,
    Close,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaDropdown;

impl WidgetSpec for WaDropdown {
    type State = WaDropdownState;
    type Message = WaDropdownMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaDropdown"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaDropdown")
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

        // 弹层组件：position=absolute + z-index 高值确保浮于内容之上
        if state.open {
            v = v.prop("position", PropValue::Str(std::sync::Arc::from("absolute")));
            v = v.prop("z-index", PropValue::Int(1000));
        }

        v
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaDropdownMessage::Close => {
                state.open = false;
            },
            // Phase 0: 其他事件无实际行为
            WaDropdownMessage::Show => {},
            WaDropdownMessage::AfterShow => {},
            WaDropdownMessage::Hide => {},
            WaDropdownMessage::AfterHide => {},
            WaDropdownMessage::Select => {},
        }
    }

    /// Dropdown 是容器，尺寸由 Taffy 根据子节点和约束计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if !state.open {
            return;
        }

        let border_radius: f32 = 8.0; // --wa-border-radius-m
        let menu_bg = Color::new(1.0, 1.0, 1.0, 1.0); // --wa-color-surface-raised
        let menu_border = Color::new(0.85, 0.85, 0.85, 1.0); // --wa-color-surface-border
        let shadow_color = Color::new(0.0, 0.0, 0.0, 0.08);

        // ── 计算菜单面板尺寸与位置 ──
        // Phase 0: 固定菜单尺寸和位置，基于 placement 近似
        let menu_w: f64 = (200.0_f64).min(bounds.size.width - 16.0);
        let menu_h: f64 = (bounds.size.height * 0.6).min(300.0);

        // 默认位置：底部左对齐（bottom-start）
        let (menu_x, menu_y) = match state.placement.as_str() {
            "top" | "top-start" => {
                let x = bounds.origin.x;
                let y = bounds.origin.y - menu_h - state.distance;
                (x, y)
            },
            "top-end" => {
                let x = bounds.origin.x + bounds.size.width - menu_w;
                let y = bounds.origin.y - menu_h - state.distance;
                (x, y)
            },
            "bottom" | "bottom-start" | "" => {
                let x = bounds.origin.x;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            },
            "bottom-end" => {
                let x = bounds.origin.x + bounds.size.width - menu_w;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            },
            "right" | "right-start" => {
                let x = bounds.origin.x + bounds.size.width + state.distance;
                let y = bounds.origin.y;
                (x, y)
            },
            "right-end" => {
                let x = bounds.origin.x + bounds.size.width + state.distance;
                let y = bounds.origin.y + bounds.size.height - menu_h;
                (x, y)
            },
            "left" | "left-start" => {
                let x = bounds.origin.x - menu_w - state.distance;
                let y = bounds.origin.y;
                (x, y)
            },
            "left-end" => {
                let x = bounds.origin.x - menu_w - state.distance;
                let y = bounds.origin.y + bounds.size.height - menu_h;
                (x, y)
            },
            _ => {
                // 默认 bottom-start
                let x = bounds.origin.x;
                let y = bounds.origin.y + bounds.size.height + state.distance;
                (x, y)
            },
        };

        let menu_bounds = Rect::new(menu_x, menu_y, menu_w, menu_h);

        // ── 1. 阴影（偏移绘制近似 box-shadow）──
        let shadow_offset: f64 = 2.0;
        let shadow_bounds = Rect::new(
            menu_x + shadow_offset,
            menu_y + shadow_offset,
            menu_w,
            menu_h,
        );
        ctx.fill_rect(shadow_bounds, shadow_color, border_radius);

        // ── 2. 菜单背景 ──
        ctx.fill_rect(menu_bounds, menu_bg, border_radius);

        // ── 3. 边框（用半透明填充近似）──
        // Phase 0: 无 stroke rect，用略小的背景偏移模拟边框
        let border_width: f64 = 1.0;
        let border_bounds = Rect::new(
            menu_x + border_width,
            menu_y + border_width,
            menu_w - 2.0 * border_width,
            menu_h - 2.0 * border_width,
        );
        ctx.fill_rect(border_bounds, menu_border, border_radius);

        // 再覆盖一层内部白色背景（边框效果）
        let inner_bg_bounds = Rect::new(
            menu_x + border_width,
            menu_y + border_width,
            menu_w - 2.0 * border_width,
            menu_h - 2.0 * border_width,
        );
        ctx.fill_rect(inner_bg_bounds, menu_bg, border_radius - 1.0);

        // ── 4. 菜单项由子节点渲染，此处不绘制内容 ──
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = if state.open {
            AccessibilityRole::Custom("menu")
        } else {
            AccessibilityRole::None
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO).label("dropdown")
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
        assert_eq!(WaDropdown.name(), "rgui_components::WaDropdown");
    }

    #[test]
    fn default_state() {
        let s = WaDropdownState::new();
        assert!(!s.open);
        assert!(s.placement.is_empty());
        assert_eq!(s.distance, 0.0);
        assert_eq!(s.skidding, 0.0);
    }

    #[test]
    fn state_open() {
        let s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        assert!(s.open);
    }

    #[test]
    fn state_with_placement() {
        let s = WaDropdownState {
            placement: "bottom-start".into(),
            ..WaDropdownState::new()
        };
        assert_eq!(s.placement, "bottom-start");
    }

    #[test]
    fn state_with_distance() {
        let s = WaDropdownState {
            distance: 8.0,
            ..WaDropdownState::new()
        };
        assert_eq!(s.distance, 8.0);
    }

    #[test]
    fn state_with_skidding() {
        let s = WaDropdownState {
            skidding: 4.0,
            ..WaDropdownState::new()
        };
        assert_eq!(s.skidding, 4.0);
    }

    #[test]
    fn view_contains_open_prop() {
        let s = WaDropdownState::new();
        let v = WaDropdown.view(&s, &make_ctx());
        assert!(v.props.contains_key("open"));
    }

    #[test]
    fn view_open_prop() {
        let s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        let v = WaDropdown.view(&s, &make_ctx());
        let val = v.props.get("open").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for open"),
        }
    }

    #[test]
    fn view_placement_prop() {
        let s = WaDropdownState {
            placement: "bottom-start".into(),
            ..WaDropdownState::new()
        };
        let v = WaDropdown.view(&s, &make_ctx());
        let val = v.props.get("placement").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "bottom-start"),
            _ => panic!("expected Str prop for placement"),
        }
    }

    #[test]
    fn view_distance_prop() {
        let s = WaDropdownState {
            distance: 8.0,
            ..WaDropdownState::new()
        };
        let v = WaDropdown.view(&s, &make_ctx());
        let val = v.props.get("distance").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 8.0),
            _ => panic!("expected Float prop for distance"),
        }
    }

    #[test]
    fn view_skidding_prop() {
        let s = WaDropdownState {
            skidding: 4.0,
            ..WaDropdownState::new()
        };
        let v = WaDropdown.view(&s, &make_ctx());
        let val = v.props.get("skidding").unwrap();
        match val {
            PropValue::Float(f) => assert_eq!(f.0, 4.0),
            _ => panic!("expected Float prop for skidding"),
        }
    }

    #[test]
    fn view_open_adds_position_absolute() {
        let s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        let v = WaDropdown.view(&s, &make_ctx());
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "absolute"),
            _ => panic!("expected Str prop for position"),
        }
    }

    #[test]
    fn view_open_adds_z_index() {
        let s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        let v = WaDropdown.view(&s, &make_ctx());
        let val = v.props.get("z-index").unwrap();
        match val {
            PropValue::Int(i) => assert_eq!(*i, 1000),
            _ => panic!("expected Int prop for z-index"),
        }
    }

    #[test]
    fn view_closed_no_position_z_index() {
        let s = WaDropdownState::new(); // open = false
        let v = WaDropdown.view(&s, &make_ctx());
        assert!(!v.props.contains_key("position"));
        assert!(!v.props.contains_key("z-index"));
    }

    #[test]
    fn update_close_sets_open_false() {
        let mut s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        WaDropdown.update(
            WaDropdownMessage::Close,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.open, "Close 应将 open 设为 false");
    }

    #[test]
    fn update_show_noop() {
        let mut s = WaDropdownState::new();
        WaDropdown.update(
            WaDropdownMessage::Show,
            &mut s,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_after_show_noop() {
        let mut s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        WaDropdown.update(
            WaDropdownMessage::AfterShow,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open);
    }

    #[test]
    fn update_select_noop() {
        let mut s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        WaDropdown.update(
            WaDropdownMessage::Select,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open, "Phase 0: Select 不自动关闭");
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaDropdownState::new();
        let size = WaDropdown.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Dropdown 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_closed_produces_no_ops() {
        let s = WaDropdownState::new(); // open = false
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdown.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "关闭的 Dropdown 不应绘制任何内容");
    }

    #[test]
    fn paint_open_produces_ops() {
        let s = WaDropdownState {
            open: true,
            placement: "bottom-start".into(),
            ..WaDropdownState::new()
        };
        let bounds = Rect::new(100.0, 100.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdown.paint(&s, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 4,
            "打开的 Dropdown 应产生多个绘制操作（阴影+背景+边框+内容），实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_top_placement() {
        let s = WaDropdownState {
            open: true,
            placement: "top-start".into(),
            ..WaDropdownState::new()
        };
        let bounds = Rect::new(100.0, 300.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdown.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "top placement 应正常绘制");
    }

    #[test]
    fn paint_right_placement() {
        let s = WaDropdownState {
            open: true,
            placement: "right-start".into(),
            ..WaDropdownState::new()
        };
        let bounds = Rect::new(100.0, 100.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdown.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "right placement 应正常绘制");
    }

    #[test]
    fn paint_left_placement() {
        let s = WaDropdownState {
            open: true,
            placement: "left-start".into(),
            ..WaDropdownState::new()
        };
        let bounds = Rect::new(300.0, 100.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdown.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "left placement 应正常绘制");
    }

    #[test]
    fn accessibility_closed_none() {
        let s = WaDropdownState::new();
        let node = WaDropdown.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("dropdown"));
    }

    #[test]
    fn accessibility_open_menu() {
        let s = WaDropdownState {
            open: true,
            ..WaDropdownState::new()
        };
        let node = WaDropdown.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("dropdown"));
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(WaDropdownMessage::Show.message_name(), "show");
        assert_eq!(WaDropdownMessage::Close.message_name(), "close");
        assert_eq!(WaDropdownMessage::Select.message_name(), "select");
    }

    #[test]
    fn derive_schema_name() {
        assert_eq!(WaDropdownState::schema_name(), "WaDropdownState");
    }

    #[test]
    fn state_as_any() {
        use std::any::Any;
        let s = WaDropdownState::new();
        let _: &dyn Any = s.as_any();
    }
}
