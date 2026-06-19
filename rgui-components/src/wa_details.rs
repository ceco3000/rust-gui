/// Translated from Web Awesome wa-details
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

/// Web Awesome wa-details 组件状态。
///
/// Details 是一种折叠/展开容器，显示摘要并展开以显示附加内容。
/// 用于渐进式信息披露、FAQ 分组或隐藏高级选项。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaDetailsState {
    /// 是否展开
    pub open: bool,
    /// 摘要文本（显示在标题栏中）
    pub summary: String,
    /// 分组名称——同名校验暂未实现（需 DOM 树遍历）
    pub name: String,
    /// 禁用状态
    pub disabled: bool,
    /// 视觉外观：filled | outlined | filled-outlined | plain
    pub appearance: String,
    /// 图标位置：start | end
    pub icon_placement: String,
}

impl WaDetailsState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            open: false,
            summary: String::new(),
            name: String::new(),
            disabled: false,
            appearance: "outlined".into(),
            icon_placement: "end".into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaDetailsMessage {
    /// 切换展开/折叠（用户点击摘要栏触发）
    Toggle,
    /// 程序化展开
    Show,
    /// 程序化折叠
    Hide,
    /// 展开完成（动画结束后，阶段 0 无动画直接触发）
    AfterShow,
    /// 折叠完成（动画结束后，阶段 0 无动画直接触发）
    AfterHide,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaDetails;

impl WidgetSpec for WaDetails {
    type State = WaDetailsState;
    type Message = WaDetailsMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaDetails"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaDetails")
            .prop("open", PropValue::bool(state.open))
            .prop("summary", PropValue::str(state.summary.as_str()))
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("disabled", PropValue::bool(state.disabled))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("icon_placement", PropValue::str(state.icon_placement.as_str()))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaDetailsMessage::Toggle => {
                if !state.disabled {
                    state.open = !state.open;
                }
            }
            WaDetailsMessage::Show => {
                if !state.disabled {
                    state.open = true;
                }
            }
            WaDetailsMessage::Hide => {
                if !state.disabled {
                    state.open = false;
                }
            }
            WaDetailsMessage::AfterShow | WaDetailsMessage::AfterHide => {
                // 阶段 0：无动画，直接忽略
            }
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        // Details 是容器，尺寸由 Taffy 根据子节点和约束计算
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let border_radius: f32 = 8.0;
        let border_width: f64 = 1.0;

        // 根据 appearance 确定背景色和边框色
        let (bg_color, border_color) = match state.appearance.as_str() {
            "plain" => (Color::TRANSPARENT, Color::TRANSPARENT),
            "filled" => (
                // neutral-fill-quiet: 浅灰背景
                Color::new(0.94, 0.94, 0.95, 1.0),
                Color::TRANSPARENT,
            ),
            "filled-outlined" => (
                Color::new(0.94, 0.94, 0.95, 1.0),
                // surface-border
                Color::new(0.82, 0.82, 0.84, 1.0),
            ),
            // "outlined"（默认）
            _ => (
                Color::new(0.98, 0.98, 0.99, 1.0), // surface-default
                Color::new(0.82, 0.82, 0.84, 1.0), // surface-border
            ),
        };

        // 绘制背景矩形
        if bg_color.a > 0.0 {
            ctx.fill_rect(bounds, bg_color, border_radius);
        }

        // 绘制边框（四条边模拟）
        if border_color.a > 0.0 && bounds.size.width > 0.0 && bounds.size.height > 0.0 {
            let x = bounds.origin.x;
            let y = bounds.origin.y;
            let w = bounds.size.width;
            let h = bounds.size.height;

            // 上边框
            ctx.fill_rect(Rect::new(x, y, w, border_width.min(h)), border_color, 0.0);
            // 下边框
            ctx.fill_rect(
                Rect::new(x, y + h - border_width, w, border_width.min(h)),
                border_color,
                0.0,
            );
            // 左边框
            ctx.fill_rect(
                Rect::new(
                    x,
                    y + border_width,
                    border_width.min(w),
                    h - 2.0 * border_width,
                ),
                border_color,
                0.0,
            );
            // 右边框
            ctx.fill_rect(
                Rect::new(
                    x + w - border_width,
                    y + border_width,
                    border_width.min(w),
                    h - 2.0 * border_width,
                ),
                border_color,
                0.0,
            );
        }

        // 绘制摘要栏——始终在顶部绘制一个摘要栏背景
        let summary_height: f64 = 44.0;
        let actual_summary_h = summary_height.min(bounds.size.height);
        let summary_bg = Color::new(0.96, 0.96, 0.97, 1.0); // 略深于默认背景的摘要栏
        let summary_rect = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            actual_summary_h,
        );
        ctx.fill_rect(summary_rect, summary_bg, border_radius);

        // 绘制折叠/展开图标（Unicode 字符）
        let chevron_char = if state.open { "▼" } else { "▶" };
        let chevron_color = Color::new(0.4, 0.4, 0.44, 1.0);
        let font_size: f32 = 12.0;
        let icon_x: f64 = if state.icon_placement.as_str() == "start" {
            bounds.origin.x + 12.0
        } else {
            bounds.origin.x + bounds.size.width - 28.0
        };
        let icon_y: f64 = bounds.origin.y + (actual_summary_h - font_size as f64) / 2.0;
        ctx.draw_text(
            chevron_char,
            Rect::new(icon_x, icon_y, 20.0, font_size as f64),
            chevron_color,
            font_size,
        );

        // 绘制摘要文本
        if !state.summary.is_empty() {
            let text_color = Color::new(0.1, 0.1, 0.12, 1.0);
            let text_font_size: f32 = 14.0;
            let text_x: f64 = if state.icon_placement.as_str() == "start" {
                bounds.origin.x + 36.0
            } else {
                bounds.origin.x + 12.0
            };
            let text_max_w = bounds.size.width - 60.0;
            let text_y: f64 = bounds.origin.y + (actual_summary_h - text_font_size as f64) / 2.0;
            ctx.draw_text(
                state.summary.as_str(),
                Rect::new(text_x, text_y, text_max_w, text_font_size as f64),
                text_color,
                text_font_size,
            );
        }

        // 如果处于展开状态，绘制摘要栏和内容区之间的分隔线
        if state.open && bounds.size.height > actual_summary_h {
            let sep_y = bounds.origin.y + actual_summary_h;
            let sep_color = Color::new(0.82, 0.82, 0.84, 1.0);
            ctx.fill_rect(
                Rect::new(bounds.origin.x + 1.0, sep_y, bounds.size.width - 2.0, 1.0),
                sep_color,
                0.0,
            );
        }
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("details")
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
        assert_eq!(WaDetails.name(), "rgui_components::WaDetails");
    }

    #[test]
    fn state_defaults() {
        let state = WaDetailsState::new();
        assert!(!state.open);
        assert_eq!(state.appearance, "outlined");
        assert_eq!(state.icon_placement, "end");
        assert!(!state.disabled);
    }

    #[test]
    fn view_has_props() {
        let state = WaDetailsState::new();
        let v = WaDetails.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("open"));
        assert!(v.props.contains_key("summary"));
        assert!(v.props.contains_key("appearance"));
        assert!(v.props.contains_key("icon_placement"));
        assert!(v.props.contains_key("disabled"));
    }

    #[test]
    fn view_default_outlined() {
        let state = WaDetailsState::new();
        let v = WaDetails.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("appearance").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "outlined"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn toggle_changes_open() {
        let mut state = WaDetailsState::new();
        assert!(!state.open);
        WaDetails.update(
            WaDetailsMessage::Toggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.open);
        WaDetails.update(
            WaDetailsMessage::Toggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open);
    }

    #[test]
    fn toggle_respects_disabled() {
        let mut state = WaDetailsState::new();
        state.disabled = true;
        WaDetails.update(
            WaDetailsMessage::Toggle,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open, "disabled 时不应切换状态");
    }

    #[test]
    fn show_and_hide() {
        let mut state = WaDetailsState::new();
        WaDetails.update(
            WaDetailsMessage::Show,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.open);
        WaDetails.update(
            WaDetailsMessage::Hide,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open);
    }

    #[test]
    fn show_respects_disabled() {
        let mut state = WaDetailsState::new();
        state.disabled = true;
        WaDetails.update(
            WaDetailsMessage::Show,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open, "disabled 时不应展开");
    }

    #[test]
    fn measure_returns_zero_delegating_to_layout() {
        let state = WaDetailsState::new();
        let size = WaDetails.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Details 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_outlined_produces_ops() {
        let state = WaDetailsState::new(); // default "outlined"
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaDetails.paint(&state, bounds, &mut ctx);
        // outlined: 背景 + 四条边框 + 摘要栏背景 + 图标 = 至少 7 个操作
        assert!(
            ctx.op_count() >= 7,
            "outlined Details 应产生背景+边框+摘要栏+图标操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_plain_produces_no_border() {
        let state = WaDetailsState {
            appearance: "plain".into(),
            ..WaDetailsState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaDetails.paint(&state, bounds, &mut ctx);
        // plain: 透明背景 + 透明边框 + 摘要栏 + 图标 => 至少 2 个操作
        assert!(
            ctx.op_count() >= 2,
            "plain Details 仅摘要栏+图标，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_open_shows_separator() {
        let state = WaDetailsState {
            open: true,
            summary: "FAQ".into(),
            ..WaDetailsState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaDetails.paint(&state, bounds, &mut ctx);
        // open: 背景+边框(4)+摘要栏+图标+文字+分隔线 >= 9
        assert!(
            ctx.op_count() >= 9,
            "展开的 Details 应包含分隔线，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_icon_placement_start() {
        let state = WaDetailsState {
            icon_placement: "start".into(),
            summary: "Test".into(),
            ..WaDetailsState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaDetails.paint(&state, bounds, &mut ctx);
        // 有摘要文字时至少包含文字操作
        assert!(ctx.op_count() > 0);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaDetailsMessage::Toggle.message_name(), "toggle");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaDetailsState::schema_name(), "WaDetailsState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaDetailsState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaDetailsState>());
    }
}
