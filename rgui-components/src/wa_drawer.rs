/// Translated from Web Awesome drawer
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// Phase 0 简化:
/// - 无动画（show/hide/pulse）
/// - 无 <dialog> HTML 元素语义（用 position: absolute + z-index 替代）
/// - 无 dismissible stack（WTI03 通过框架 handle_click 发送 Close）
/// - 无键盘 Escape 处理（由框架事件系统后续支持）
/// - 无 light dismiss 行为（由 WTI03 处理）
/// - 无 header-actions slot 渲染（仅绘制基础 header）
/// - RTL 镜像 → 跳过
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

/// Web Awesome wa-drawer 组件状态。
///
/// Drawer 是弹层容器，从视口边缘滑入以展示附加选项和信息，
/// 无需导航离开。适用于导航菜单、筛选器和辅助内容。
///
/// Phase 0 简化项：
/// - `withFooter`（SSR 专属）→ 跳过
/// - 动画状态（show/hide/pulse CSS 类）→ Phase 2
/// - LocalizeController → 硬编码英文
/// - ElementInternals / HasSlotController → children 遍历
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaDrawerState {
    /// 抽屉是否打开
    pub open: bool,
    /// 标题文本。若为空也不设 label slot，使用零宽字符占位
    pub label: String,
    /// 打开方向：top / end（右侧，默认）/ bottom / start（左侧）
    pub placement: String,
    /// 隐藏 header（包括标题和关闭按钮）
    pub without_header: bool,
    /// 点击背景关闭抽屉（WTI03 处理）
    pub light_dismiss: bool,
}

impl WaDrawerState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            open: false,
            label: String::new(),
            placement: "end".into(),
            without_header: false,
            light_dismiss: false,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// Drawer 事件。
///
/// - `Show` — 抽屉打开前（wa-show，可取消）
/// - `AfterShow` — 抽屉打开后
/// - `Hide` — 请求关闭（wa-hide，可取消）
/// - `AfterHide` — 关闭后
/// - `Close` — WTI03 框架发送的关闭指令（点击外部或程序化关闭）
///
/// Phase 0：除 Close 外所有事件无实际行为，保留占位供未来实现。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaDrawerMessage {
    Show,
    AfterShow,
    Hide,
    AfterHide,
    Close,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaDrawer;

impl WidgetSpec for WaDrawer {
    type State = WaDrawerState;
    type Message = WaDrawerMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaDrawer"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaDrawer")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("open", PropValue::Bool(state.open))
            .prop("placement", PropValue::str(state.placement.as_str()));

        if state.without_header {
            v = v.prop("without-header", PropValue::Bool(true));
        }
        if state.light_dismiss {
            v = v.prop("light-dismiss", PropValue::Bool(true));
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
            WaDrawerMessage::Close => {
                state.open = false;
            }
            // Phase 0: 其他事件无实际行为
            WaDrawerMessage::Show => {}
            WaDrawerMessage::AfterShow => {}
            WaDrawerMessage::Hide => {}
            WaDrawerMessage::AfterHide => {}
        }
    }

    /// Drawer 是容器，尺寸由 Taffy 根据子节点和约束计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if !state.open {
            return;
        }

        let border_radius: f32 = 12.0; // --wa-panel-border-radius
        let backdrop_color = Color::new(0.0, 0.0, 0.0, 0.25); // --wa-color-overlay-modal
        let panel_bg = Color::new(1.0, 1.0, 1.0, 1.0); // --wa-color-surface-raised
        let header_border_color = Color::new(0.88, 0.88, 0.88, 1.0);
        let text_color = Color::new(0.13, 0.13, 0.13, 1.0);
        let close_color = Color::new(0.45, 0.45, 0.45, 1.0);

        // ── 绘制背景遮罩（覆盖整个 bounds）──
        ctx.fill_rect(bounds, backdrop_color, 0.0);

        // ── 计算面板尺寸与位置（根据 placement）──
        // --size = 400px (≈25rem)
        let drawer_size: f64 = 400.0;
        let (panel_x, panel_y, panel_w, panel_h) = match state.placement.as_str() {
            "top" => {
                let h = drawer_size.min(bounds.size.height - 64.0).max(100.0);
                (bounds.origin.x, bounds.origin.y, bounds.size.width, h)
            }
            "end" => {
                let w = drawer_size.min(bounds.size.width - 64.0).max(200.0);
                (
                    bounds.origin.x + bounds.size.width - w,
                    bounds.origin.y,
                    w,
                    bounds.size.height,
                )
            }
            "bottom" => {
                let h = drawer_size.min(bounds.size.height - 64.0).max(100.0);
                (
                    bounds.origin.x,
                    bounds.origin.y + bounds.size.height - h,
                    bounds.size.width,
                    h,
                )
            }
            "start" => {
                let w = drawer_size.min(bounds.size.width - 64.0).max(200.0);
                (bounds.origin.x, bounds.origin.y, w, bounds.size.height)
            }
            _ => {
                // fallback: end
                let w = drawer_size.min(bounds.size.width - 64.0).max(200.0);
                (
                    bounds.origin.x + bounds.size.width - w,
                    bounds.origin.y,
                    w,
                    bounds.size.height,
                )
            }
        };

        let panel_bounds = Rect::new(panel_x, panel_y, panel_w, panel_h);

        // ── 绘制面板阴影（dark offset rect）──
        let shadow_color = Color::new(0.0, 0.0, 0.0, 0.10);
        let shadow_offset: f64 = 2.0;
        let shadow_bounds = Rect::new(
            panel_x + shadow_offset,
            panel_y + shadow_offset,
            panel_w,
            panel_h,
        );
        ctx.fill_rect(shadow_bounds, shadow_color, border_radius);

        // ── 绘制面板背景 ──
        ctx.fill_rect(panel_bounds, panel_bg, border_radius);

        // ── 绘制 Header ──
        if !state.without_header {
            let header_h: f64 = 52.0;

            // Header 底部分隔线
            let line_y = panel_y + header_h - 1.0;
            let line_bounds = Rect::new(panel_x + 16.0, line_y, panel_w - 32.0, 1.0);
            ctx.fill_rect(line_bounds, header_border_color, 0.0);

            // 标题文本
            let display_label = if state.label.is_empty() {
                "Drawer"
            } else {
                state.label.as_str()
            };

            let title_font_size: f32 = 18.0;
            let title_bounds = Rect::new(
                panel_x + 20.0,
                panel_y,
                panel_w - 80.0,
                header_h,
            );
            ctx.draw_text(display_label, title_bounds, text_color, title_font_size);

            // 关闭按钮 X（右上角）
            let close_btn_size: f64 = 32.0;
            let close_x = panel_x + panel_w - close_btn_size - 8.0;
            let close_y = panel_y + (header_h - close_btn_size) / 2.0;
            let close_bounds = Rect::new(close_x, close_y, close_btn_size, close_btn_size);

            ctx.fill_rect(close_bounds, Color::TRANSPARENT, 4.0);
            let close_font_size: f32 = 18.0;
            ctx.draw_text("\u{2715}", close_bounds, close_color, close_font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = if state.open {
            AccessibilityRole::Dialog
        } else {
            AccessibilityRole::None
        };
        let label = if state.label.is_empty() {
            "drawer"
        } else {
            state.label.as_str()
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO).label(label)
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
        assert_eq!(WaDrawer.name(), "rgui_components::WaDrawer");
    }

    #[test]
    fn default_state() {
        let s = WaDrawerState::new();
        assert!(!s.open);
        assert!(s.label.is_empty());
        assert_eq!(s.placement, "end");
        assert!(!s.without_header);
        assert!(!s.light_dismiss);
    }

    #[test]
    fn state_open() {
        let s = WaDrawerState {
            open: true,
            ..WaDrawerState::new()
        };
        assert!(s.open);
    }

    #[test]
    fn state_with_label() {
        let s = WaDrawerState {
            label: "Navigation".into(),
            ..WaDrawerState::new()
        };
        assert_eq!(s.label, "Navigation");
    }

    #[test]
    fn state_placement_top() {
        let s = WaDrawerState {
            placement: "top".into(),
            ..WaDrawerState::new()
        };
        assert_eq!(s.placement, "top");
    }

    #[test]
    fn state_placement_start() {
        let s = WaDrawerState {
            placement: "start".into(),
            ..WaDrawerState::new()
        };
        assert_eq!(s.placement, "start");
    }

    #[test]
    fn state_without_header() {
        let s = WaDrawerState {
            without_header: true,
            ..WaDrawerState::new()
        };
        assert!(s.without_header);
    }

    #[test]
    fn state_light_dismiss() {
        let s = WaDrawerState {
            light_dismiss: true,
            ..WaDrawerState::new()
        };
        assert!(s.light_dismiss);
    }

    #[test]
    fn view_contains_core_props() {
        let s = WaDrawerState::new();
        let v = WaDrawer.view(&s, &make_ctx());
        assert!(v.props.contains_key("label"));
        assert!(v.props.contains_key("open"));
        assert!(v.props.contains_key("placement"));
    }

    #[test]
    fn view_label_prop() {
        let s = WaDrawerState {
            label: "Settings".into(),
            ..WaDrawerState::new()
        };
        let v = WaDrawer.view(&s, &make_ctx());
        let val = v.props.get("label").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "Settings"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_placement_prop() {
        let s = WaDrawerState {
            placement: "bottom".into(),
            ..WaDrawerState::new()
        };
        let v = WaDrawer.view(&s, &make_ctx());
        let val = v.props.get("placement").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "bottom"),
            _ => panic!("expected Str prop for placement"),
        }
    }

    #[test]
    fn view_open_adds_position_absolute() {
        let s = WaDrawerState {
            open: true,
            ..WaDrawerState::new()
        };
        let v = WaDrawer.view(&s, &make_ctx());
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "absolute"),
            _ => panic!("expected Str prop for position"),
        }
    }

    #[test]
    fn view_open_adds_z_index() {
        let s = WaDrawerState {
            open: true,
            ..WaDrawerState::new()
        };
        let v = WaDrawer.view(&s, &make_ctx());
        let val = v.props.get("z-index").unwrap();
        match val {
            PropValue::Int(i) => assert_eq!(*i, 1000),
            _ => panic!("expected Int prop for z-index"),
        }
    }

    #[test]
    fn view_without_header_prop() {
        let s = WaDrawerState {
            without_header: true,
            ..WaDrawerState::new()
        };
        let v = WaDrawer.view(&s, &make_ctx());
        let val = v.props.get("without-header").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for without-header"),
        }
    }

    #[test]
    fn view_light_dismiss_prop() {
        let s = WaDrawerState {
            light_dismiss: true,
            ..WaDrawerState::new()
        };
        let v = WaDrawer.view(&s, &make_ctx());
        let val = v.props.get("light-dismiss").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for light-dismiss"),
        }
    }

    #[test]
    fn update_close_sets_open_false() {
        let mut s = WaDrawerState {
            open: true,
            ..WaDrawerState::new()
        };
        WaDrawer.update(
            WaDrawerMessage::Close,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.open, "Close 应将 open 设为 false");
    }

    #[test]
    fn update_show_keeps_state() {
        let mut s = WaDrawerState {
            open: true,
            ..WaDrawerState::new()
        };
        WaDrawer.update(
            WaDrawerMessage::Show,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open, "Show 不应改变 open 状态");
    }

    #[test]
    fn update_after_show_noop() {
        let mut s = WaDrawerState::new();
        WaDrawer.update(
            WaDrawerMessage::AfterShow,
            &mut s,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_hide_noop() {
        let mut s = WaDrawerState {
            open: true,
            ..WaDrawerState::new()
        };
        WaDrawer.update(
            WaDrawerMessage::Hide,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open, "Phase 0: Hide 事件不自动关闭");
    }

    #[test]
    fn update_after_hide_noop() {
        let mut s = WaDrawerState::new();
        WaDrawer.update(
            WaDrawerMessage::AfterHide,
            &mut s,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaDrawerState::new();
        let size = WaDrawer.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Drawer 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_closed_produces_no_ops() {
        let s = WaDrawerState::new(); // open = false
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDrawer.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "关闭的 Drawer 不应绘制任何内容");
    }

    #[test]
    fn paint_open_end_placement_produces_ops() {
        let s = WaDrawerState {
            open: true,
            label: "Menu".into(),
            placement: "end".into(),
            ..WaDrawerState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDrawer.paint(&s, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 4,
            "打开的 Drawer 应产生多个绘制操作（背景+阴影+面板+header分隔线+标题+关闭按钮），实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_open_top_placement() {
        let s = WaDrawerState {
            open: true,
            label: "Filters".into(),
            placement: "top".into(),
            ..WaDrawerState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 1024.0, 768.0);
        let mut ctx = PaintContext::new(bounds);
        WaDrawer.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "top placement 应正常绘制");
    }

    #[test]
    fn paint_open_bottom_placement() {
        let s = WaDrawerState {
            open: true,
            placement: "bottom".into(),
            ..WaDrawerState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 1024.0, 768.0);
        let mut ctx = PaintContext::new(bounds);
        WaDrawer.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "bottom placement 应正常绘制");
    }

    #[test]
    fn paint_open_start_placement() {
        let s = WaDrawerState {
            open: true,
            placement: "start".into(),
            ..WaDrawerState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 1024.0, 768.0);
        let mut ctx = PaintContext::new(bounds);
        WaDrawer.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "start placement 应正常绘制");
    }

    #[test]
    fn paint_open_without_header_less_ops() {
        let s = WaDrawerState {
            open: true,
            without_header: true,
            ..WaDrawerState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDrawer.paint(&s, bounds, &mut ctx);
        // 背景 + 阴影 + 面板（无 header 相关内容）
        assert!(
            ctx.op_count() >= 3,
            "without-header Drawer 仍应有基础绘制，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn accessibility_closed() {
        let s = WaDrawerState::new();
        let node = WaDrawer.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.role, AccessibilityRole::None);
    }

    #[test]
    fn accessibility_open() {
        let s = WaDrawerState {
            open: true,
            label: "Nav".into(),
            ..WaDrawerState::new()
        };
        let node = WaDrawer.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.role, AccessibilityRole::Dialog);
    }

    #[test]
    fn accessibility_fallback_label() {
        let s = WaDrawerState {
            open: true,
            ..WaDrawerState::new()
        };
        let node = WaDrawer.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("drawer"));
    }

    // --- Derive macro smoke tests ---

    #[test]
    fn wa_drawer_message_is_app_message() {
        // 验证 WaDrawerMessage 实现了 AppMessage trait（由派生宏生成）
        fn assert_app_message<T: AppMessage>() {}
        assert_app_message::<WaDrawerMessage>();
    }

    #[test]
    fn wa_drawer_state_is_persist_state() {
        // 验证 WaDrawerState 实现了 PersistState trait（由派生宏生成）
        fn assert_persist_state<T: PersistState>() {}
        assert_persist_state::<WaDrawerState>();
    }
}
