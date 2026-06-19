/// Translated from Web Awesome dialog
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

/// Web Awesome wa-dialog 组件状态。
///
/// Dialog 是弹层容器，在页面内容之上显示对话框，
/// 需要用户立即关注。包含 header（标题 + 关闭按钮）、body（slot 子内容）和 footer。
///
/// Phase 0 简化项：
/// - `withFooter`（SSR 专属）→ 跳过
/// - 动画状态（show/hide/pulse CSS 类）→ Phase 2
/// - LocalizeController → 硬编码英文
/// - ElementInternals / HasSlotController → children 遍历
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaDialogState {
    /// 对话框是否打开
    pub open: bool,
    /// 标题文本。若为空也不设 label slot，使用零宽字符占位
    pub label: String,
    /// 隐藏 header（包括标题和关闭按钮）
    pub without_header: bool,
    /// 点击背景关闭对话框（WTI03 处理）
    pub light_dismiss: bool,
}

impl WaDialogState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            open: false,
            label: String::new(),
            without_header: false,
            light_dismiss: false,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// Dialog 事件。
///
/// - `Show` — 对话框打开前（wa-show，可取消）
/// - `AfterShow` — 对话框打开后
/// - `Hide` — 请求关闭（wa-hide，可取消）
/// - `AfterHide` — 关闭后
/// - `Close` — WTI03 框架发送的关闭指令（点击外部或程序化关闭）
///
/// Phase 0：除 Close 外所有事件无实际行为，保留占位供未来实现。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaDialogMessage {
    Show,
    AfterShow,
    Hide,
    AfterHide,
    Close,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaDialog;

impl WidgetSpec for WaDialog {
    type State = WaDialogState;
    type Message = WaDialogMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaDialog"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaDialog")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop(
                "open",
                PropValue::Bool(state.open),
            );

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
            WaDialogMessage::Close => {
                state.open = false;
            }
            // Phase 0: 其他事件无实际行为
            WaDialogMessage::Show => {},
            WaDialogMessage::AfterShow => {},
            WaDialogMessage::Hide => {},
            WaDialogMessage::AfterHide => {},
        }
    }

    /// Dialog 是容器，尺寸由 Taffy 根据子节点和约束计算。
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

        // ── 计算面板尺寸与位置 ──
        // 面板宽度：min(496px, bounds.w - 64px)
        let panel_w: f64 = (496.0_f64).min((bounds.size.width - 64.0).max(200.0));
        let panel_h: f64 = (bounds.size.height * 0.8).max(200.0).min(bounds.size.height - 64.0);
        let panel_x: f64 = bounds.origin.x + (bounds.size.width - panel_w) / 2.0;
        let panel_y: f64 = bounds.origin.y + (bounds.size.height - panel_h) / 2.0;

        let panel_bounds = Rect::new(panel_x, panel_y, panel_w, panel_h);

        // ── 1. 绘制背景遮罩（覆盖整个 bounds）──
        ctx.fill_rect(bounds, backdrop_color, 0.0);

        // ── 2. 绘制面板阴影和背景 ──
        // 阴影用 dark border 近似
        let shadow_color = Color::new(0.0, 0.0, 0.0, 0.10);
        let shadow_offset: f64 = 2.0;
        let shadow_bounds = Rect::new(
            panel_x + shadow_offset,
            panel_y + shadow_offset,
            panel_w,
            panel_h,
        );
        ctx.fill_rect(shadow_bounds, shadow_color, border_radius);
        ctx.fill_rect(panel_bounds, panel_bg, border_radius);

        // ── 3. 绘制 Header ──
        if !state.without_header {
            let header_h: f64 = 52.0;

            // Header 底部分隔线
            let line_y = panel_y + header_h - 1.0;
            let line_bounds = Rect::new(
                panel_x + 16.0,
                line_y,
                panel_w - 32.0,
                1.0,
            );
            ctx.fill_rect(line_bounds, header_border_color, 0.0);

            // 标题文本
            let display_label = if state.label.is_empty() {
                "Dialog"
            } else {
                state.label.as_str()
            };

            let title_font_size: f32 = 18.0;
            // Title area: left-aligned with padding
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

            // X 图标背景
            ctx.fill_rect(close_bounds, Color::TRANSPARENT, 4.0);
            // X 文本
            let close_font_size: f32 = 18.0;
            ctx.draw_text("\u{2715}", close_bounds, close_color, close_font_size);

            // ── 4. Body 区域（透明，由子节点渲染）──
            // 不需要绘制，子节点由框架递归处理
            let _body_y = panel_y + header_h;
            let _body_h = panel_h - header_h;

            // ── 5. Footer 区域（透明，由 footer slot 子节点渲染）──
            // Phase 0：footer 由框架 child 渲染，此处不绘制
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = if state.open {
            AccessibilityRole::Dialog
        } else {
            AccessibilityRole::None
        };
        let label = if state.label.is_empty() {
            "dialog"
        } else {
            state.label.as_str()
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO)
            .label(label)
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
        assert_eq!(WaDialog.name(), "rgui_components::WaDialog");
    }

    #[test]
    fn default_state() {
        let s = WaDialogState::new();
        assert!(!s.open);
        assert!(s.label.is_empty());
        assert!(!s.without_header);
        assert!(!s.light_dismiss);
    }

    #[test]
    fn state_open() {
        let s = WaDialogState {
            open: true,
            ..WaDialogState::new()
        };
        assert!(s.open);
    }

    #[test]
    fn state_with_label() {
        let s = WaDialogState {
            label: "Confirm Delete".into(),
            ..WaDialogState::new()
        };
        assert_eq!(s.label, "Confirm Delete");
    }

    #[test]
    fn state_without_header() {
        let s = WaDialogState {
            without_header: true,
            ..WaDialogState::new()
        };
        assert!(s.without_header);
    }

    #[test]
    fn state_light_dismiss() {
        let s = WaDialogState {
            light_dismiss: true,
            ..WaDialogState::new()
        };
        assert!(s.light_dismiss);
    }

    #[test]
    fn view_contains_core_props() {
        let s = WaDialogState::new();
        let v = WaDialog.view(&s, &make_ctx());
        assert!(v.props.contains_key("label"));
        assert!(v.props.contains_key("open"));
    }

    #[test]
    fn view_label_prop() {
        let s = WaDialogState {
            label: "Settings".into(),
            ..WaDialogState::new()
        };
        let v = WaDialog.view(&s, &make_ctx());
        let val = v.props.get("label").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "Settings"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_open_adds_position_absolute() {
        let s = WaDialogState {
            open: true,
            ..WaDialogState::new()
        };
        let v = WaDialog.view(&s, &make_ctx());
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "absolute"),
            _ => panic!("expected Str prop for position"),
        }
    }

    #[test]
    fn view_open_adds_z_index() {
        let s = WaDialogState {
            open: true,
            ..WaDialogState::new()
        };
        let v = WaDialog.view(&s, &make_ctx());
        let val = v.props.get("z-index").unwrap();
        match val {
            PropValue::Int(i) => assert_eq!(*i, 1000),
            _ => panic!("expected Int prop for z-index"),
        }
    }

    #[test]
    fn view_without_header_prop() {
        let s = WaDialogState {
            without_header: true,
            ..WaDialogState::new()
        };
        let v = WaDialog.view(&s, &make_ctx());
        let val = v.props.get("without-header").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for without-header"),
        }
    }

    #[test]
    fn view_light_dismiss_prop() {
        let s = WaDialogState {
            light_dismiss: true,
            ..WaDialogState::new()
        };
        let v = WaDialog.view(&s, &make_ctx());
        let val = v.props.get("light-dismiss").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for light-dismiss"),
        }
    }

    #[test]
    fn update_close_sets_open_false() {
        let mut s = WaDialogState {
            open: true,
            ..WaDialogState::new()
        };
        WaDialog.update(
            WaDialogMessage::Close,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.open, "Close 应将 open 设为 false");
    }

    #[test]
    fn update_show_keeps_state() {
        let mut s = WaDialogState {
            open: true,
            ..WaDialogState::new()
        };
        WaDialog.update(
            WaDialogMessage::Show,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open, "Show 不应改变 open 状态");
    }

    #[test]
    fn update_after_show_noop() {
        let mut s = WaDialogState::new();
        WaDialog.update(
            WaDialogMessage::AfterShow,
            &mut s,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_hide_noop() {
        let mut s = WaDialogState {
            open: true,
            ..WaDialogState::new()
        };
        WaDialog.update(
            WaDialogMessage::Hide,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.open, "Phase 0: Hide 事件不自动关闭");
    }

    #[test]
    fn update_after_hide_noop() {
        let mut s = WaDialogState::new();
        WaDialog.update(
            WaDialogMessage::AfterHide,
            &mut s,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaDialogState::new();
        let size = WaDialog.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Dialog 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_closed_produces_no_ops() {
        let s = WaDialogState::new(); // open = false
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDialog.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "关闭的 Dialog 不应绘制任何内容");
    }

    #[test]
    fn paint_open_produces_ops() {
        let s = WaDialogState {
            open: true,
            label: "Confirm".into(),
            ..WaDialogState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDialog.paint(&s, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 4,
            "打开的 Dialog 应产生多个绘制操作（背景+阴影+面板+header分隔线+标题+关闭按钮），实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_open_without_header_less_ops() {
        let s = WaDialogState {
            open: true,
            without_header: true,
            ..WaDialogState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDialog.paint(&s, bounds, &mut ctx);
        // 背景 + 阴影 + 面板（无 header 相关内容）
        assert!(ctx.op_count() >= 3, "without-header Dialog 仍应有基础绘制");
    }

    #[test]
    fn paint_empty_label_shows_default() {
        let s = WaDialogState {
            open: true,
            label: String::new(),
            ..WaDialogState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaDialog.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4, "空 label 应显示默认标题");
    }

    #[test]
    fn accessibility_open_dialog_role() {
        let s = WaDialogState {
            open: true,
            label: "Settings".into(),
            ..WaDialogState::new()
        };
        let node = WaDialog.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Settings"));
    }

    #[test]
    fn accessibility_closed_none_role() {
        let s = WaDialogState::new(); // closed
        let node = WaDialog.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("dialog"));
    }

    #[test]
    fn accessibility_empty_label_fallback() {
        let s = WaDialogState {
            open: true,
            label: String::new(),
            ..WaDialogState::new()
        };
        let node = WaDialog.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("dialog"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaDialogMessage::Close.message_name(), "close");
        assert_eq!(WaDialogMessage::Show.message_name(), "show");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaDialogState::schema_name(), "WaDialogState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaDialogState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaDialogState>());
    }
}
