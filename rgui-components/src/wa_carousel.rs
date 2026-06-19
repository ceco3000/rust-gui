/// Translated from Web Awesome wa-carousel
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

/// Web Awesome wa-carousel 组件状态。
///
/// Carousel 沿水平或垂直轴显示一系列内容幻灯片，支持导航按钮和分页指示器。
///
/// Phase 0 简化：静态渲染导航按钮和分页圆点，跳过自动播放、鼠标拖拽、
/// 键盘导航、滚动交互和循环克隆。子节点（WaCarouselItem）由布局引擎排列。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaCarouselState {
    /// 布局方向：horizontal | vertical。
    pub orientation: String,
    /// 是否显示前进/后退导航按钮。
    pub navigation: bool,
    /// 是否显示分页指示器圆点。
    pub pagination: bool,
    /// 同时可见的幻灯片数量。
    pub slides_per_page: u32,
    /// 每次前进/后退跳过的幻灯片数量。
    pub slides_per_move: u32,
    /// 幻灯片总数（用于渲染分页圆点）。
    pub slides: u32,
    /// 当前幻灯片索引。
    pub current_slide: u32,
    /// 激活的幻灯片索引。
    pub active_slide: u32,
    /// Phase 0：loop 模式暂不实现滚动逻辑，仅保留属性。
    pub r#loop: bool,
    /// Phase 0：自动播放暂不实现。
    pub autoplay: bool,
    /// Phase 0：鼠标拖拽暂不实现。
    pub mouse_dragging: bool,
}

impl WaCarouselState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            orientation: "horizontal".into(),
            navigation: false,
            pagination: false,
            slides_per_page: 1,
            slides_per_move: 1,
            slides: 0,
            current_slide: 0,
            active_slide: 0,
            r#loop: false,
            autoplay: false,
            mouse_dragging: false,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaCarouselMessage {
    /// wa-slide-change 事件——活动幻灯片变化。
    SlideChange,
    /// 占位消息。
    NoOp,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaCarousel;

impl WidgetSpec for WaCarousel {
    type State = WaCarouselState;
    type Message = WaCarouselMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaCarousel"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaCarousel")
            .prop("orientation", PropValue::str(state.orientation.as_str()))
            .prop("navigation", PropValue::Bool(state.navigation))
            .prop("pagination", PropValue::Bool(state.pagination))
            .prop(
                "slides-per-page",
                PropValue::Int(state.slides_per_page as i64),
            )
            .prop(
                "slides-per-move",
                PropValue::Int(state.slides_per_move as i64),
            )
            .prop("slides", PropValue::Int(state.slides as i64))
            .prop("current-slide", PropValue::Int(state.current_slide as i64))
            .prop("active-slide", PropValue::Int(state.active_slide as i64))
            .prop("loop", PropValue::Bool(state.r#loop))
            .prop("autoplay", PropValue::Bool(state.autoplay))
            .prop("mouse-dragging", PropValue::Bool(state.mouse_dragging))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaCarouselMessage::SlideChange => {
                // Phase 0：不做滚动切换
            },
            WaCarouselMessage::NoOp => {},
        }
    }

    /// Carousel 是容器组件，尺寸由 Taffy 布局（子节点驱动）。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let h: f64 = bounds.size.height;
        let w: f64 = bounds.size.width;

        if h < 8.0 || w < 8.0 {
            return;
        }

        let is_horizontal = state.orientation != "vertical";

        // 导航按钮区域宽度
        let nav_btn_w: f64 = if state.navigation { 32.0 } else { 0.0 };
        let _nav_area_w: f64 = nav_btn_w * 2.0; // prev + next

        // 分页区域高度
        let pagination_h: f64 = if state.pagination { 24.0 } else { 0.0 };

        // 背景色
        let bg = Color::new(0.97, 0.97, 0.97, 1.0); // 浅灰
        let nav_color = Color::new(0.33, 0.33, 0.38, 1.0); // 导航按钮颜色
        let nav_disabled_color = Color::new(0.75, 0.75, 0.75, 1.0); // 禁用导航
        let dot_color = Color::new(0.82, 0.82, 0.85, 1.0); // 默认圆点
        let dot_active_color = Color::new(0.06, 0.41, 0.91, 1.0); // 激活圆点

        // ── 背景 ──
        ctx.fill_rect(bounds, bg, 0.0);

        let font_size: f32 = 18.0;

        // ── 导航按钮 ──
        if state.navigation {
            let nav_y = bounds.origin.y;
            let nav_h = if state.pagination {
                h - pagination_h
            } else {
                h
            };

            let can_prev = state.active_slide > 0 || state.r#loop;
            let can_next = if state.slides > 0 {
                state.active_slide + state.slides_per_page < state.slides || state.r#loop
            } else {
                false
            };

            let prev_color = if can_prev {
                nav_color
            } else {
                nav_disabled_color
            };
            let next_color = if can_next {
                nav_color
            } else {
                nav_disabled_color
            };

            // 上一页按钮 ◀
            if is_horizontal {
                let prev_rect = Rect::new(bounds.origin.x, nav_y, nav_btn_w, nav_h);
                ctx.draw_text("◀", prev_rect, prev_color, font_size);
            }

            // 下一页按钮 ▶
            if is_horizontal {
                let next_rect = Rect::new(bounds.origin.x + w - nav_btn_w, nav_y, nav_btn_w, nav_h);
                ctx.draw_text("▶", next_rect, next_color, font_size);
            } else {
                // 垂直方向：上 = ◀ 旋转概念，用 ▲ ▼
                let prev_rect = Rect::new(bounds.origin.x, nav_y, nav_btn_w, nav_h);
                ctx.draw_text("▲", prev_rect, prev_color, font_size);

                let next_rect = Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + nav_h - nav_btn_w,
                    nav_btn_w,
                    nav_btn_w,
                );
                ctx.draw_text("▼", next_rect, next_color, font_size);
            }
        }

        // ── 分页圆点 ──
        if state.pagination && state.slides > 0 {
            let slides_per_page = state.slides_per_page.max(1);
            let slides_per_move = state.slides_per_move.max(1);

            // 计算页数（WA 公式简化）
            let page_count = if state.r#loop {
                (state.slides as f64 / slides_per_move as f64).ceil() as u32
            } else if state.slides > slides_per_page {
                ((state.slides - slides_per_page) as f64 / slides_per_move as f64).ceil() as u32 + 1
            } else {
                1
            };

            let current_page = if slides_per_move > 0 {
                state.active_slide / slides_per_move
            } else {
                0
            };

            let dot_size: f64 = 8.0;
            let dot_gap: f64 = 4.0;
            let total_dots_w: f64 =
                page_count as f64 * dot_size + (page_count.saturating_sub(1)) as f64 * dot_gap;
            let start_x = bounds.origin.x + (w - total_dots_w) / 2.0;
            let dot_y = bounds.origin.y + h - pagination_h + (pagination_h - dot_size) / 2.0;

            for i in 0..page_count {
                let color = if i == current_page {
                    dot_active_color
                } else {
                    dot_color
                };
                let dx = start_x + i as f64 * (dot_size + dot_gap);
                let dot_rect = Rect::new(dx, dot_y, dot_size, dot_size);
                let radius: f32 = (dot_size / 2.0) as f32;
                ctx.fill_rect(dot_rect, color, radius);
            }
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let _ = state;
        AccessibilityNode::new(
            WidgetId::from_u64(0),
            AccessibilityRole::Custom("region"),
            Rect::ZERO,
        )
        .label("carousel")
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
        let state = WaCarouselState::new();
        assert_eq!(state.orientation, "horizontal");
        assert!(!state.navigation);
        assert!(!state.pagination);
        assert_eq!(state.slides_per_page, 1);
        assert_eq!(state.slides_per_move, 1);
        assert_eq!(state.slides, 0);
        assert!(!state.r#loop);
        assert!(!state.autoplay);
        assert!(!state.mouse_dragging);
    }

    #[test]
    fn state_with_navigation_and_pagination() {
        let state = WaCarouselState {
            navigation: true,
            pagination: true,
            slides: 3,
            slides_per_page: 1,
            slides_per_move: 1,
            ..WaCarouselState::new()
        };
        assert!(state.navigation);
        assert!(state.pagination);
        assert_eq!(state.slides, 3);
    }

    #[test]
    fn state_vertical_orientation() {
        let state = WaCarouselState {
            orientation: "vertical".into(),
            ..WaCarouselState::new()
        };
        assert_eq!(state.orientation, "vertical");
    }

    #[test]
    fn state_loop_enabled() {
        let state = WaCarouselState {
            r#loop: true,
            ..WaCarouselState::new()
        };
        assert!(state.r#loop);
    }

    #[test]
    fn state_multi_slide_per_page() {
        let state = WaCarouselState {
            slides_per_page: 3,
            slides_per_move: 1,
            slides: 9,
            ..WaCarouselState::new()
        };
        assert_eq!(state.slides_per_page, 3);
        assert_eq!(state.slides, 9);
    }

    #[test]
    fn message_noop() {
        let mut state = WaCarouselState::new();
        let mut ctx = UpdateContext::default();
        WaCarousel.update(WaCarouselMessage::NoOp, &mut state, &mut ctx);
    }

    #[test]
    fn message_slide_change_noop() {
        let mut state = WaCarouselState {
            active_slide: 2,
            ..WaCarouselState::new()
        };
        let mut ctx = UpdateContext::default();
        WaCarousel.update(WaCarouselMessage::SlideChange, &mut state, &mut ctx);
        // Phase 0 不修改状态
        assert_eq!(state.active_slide, 2);
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaCarouselState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaCarousel.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_no_navigation_no_pagination_produces_bg() {
        let state = WaCarouselState::new(); // navigation=false, pagination=false
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarousel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 1, "仅背景 1 个 FillRect");
    }

    #[test]
    fn paint_with_navigation_produces_buttons() {
        let state = WaCarouselState {
            navigation: true,
            ..WaCarouselState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarousel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 2, "应有 2 个 DrawText（prev + next 按钮）");
    }

    #[test]
    fn paint_with_pagination_produces_dots() {
        let state = WaCarouselState {
            pagination: true,
            slides: 3,
            slides_per_page: 1,
            slides_per_move: 1,
            ..WaCarouselState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarousel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        // 1 bg + 3 dots = 4
        assert_eq!(fill_count, 4, "背景 + 3 个分页圆点 = 4");
    }

    #[test]
    fn paint_with_navigation_and_pagination() {
        let state = WaCarouselState {
            navigation: true,
            pagination: true,
            slides: 2,
            slides_per_page: 1,
            slides_per_move: 1,
            ..WaCarouselState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarousel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 3, "1 bg + 2 dots = 3");
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 2, "2 navigation buttons");
    }

    #[test]
    fn paint_too_small_bounds_returns_early() {
        let state = WaCarouselState {
            navigation: true,
            pagination: true,
            slides: 5,
            ..WaCarouselState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 3.0, 3.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarousel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "过小 bounds 应提前返回");
    }

    #[test]
    fn paint_vertical_orientation() {
        let state = WaCarouselState {
            orientation: "vertical".into(),
            navigation: true,
            ..WaCarouselState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarousel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 2, "垂直模式也应有 2 个导航按钮");
    }

    #[test]
    fn paint_active_dot_highlighted() {
        let state = WaCarouselState {
            pagination: true,
            slides: 5,
            slides_per_page: 1,
            slides_per_move: 1,
            active_slide: 2,
            ..WaCarouselState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
        let mut ctx = PaintContext::new(bounds);
        WaCarousel.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 1 bg + 5 dots = 6 FillRect
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 6, "5 dots + 1 bg = 6");
    }

    #[test]
    fn accessibility_label() {
        let state = WaCarouselState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaCarousel.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("carousel"));
    }

    #[test]
    fn view_contains_props() {
        let state = WaCarouselState {
            orientation: "vertical".into(),
            navigation: true,
            pagination: true,
            slides_per_page: 2,
            slides_per_move: 1,
            slides: 5,
            active_slide: 3,
            r#loop: true,
            ..WaCarouselState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaCarousel.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaCarousel");

        if let Some(PropValue::Str(o)) = view.props.get("orientation") {
            assert_eq!(o.as_ref(), "vertical");
        } else {
            panic!("Expected Str prop 'orientation'");
        }

        if let Some(PropValue::Bool(n)) = view.props.get("navigation") {
            assert!(*n);
        } else {
            panic!("Expected Bool prop 'navigation'");
        }

        if let Some(PropValue::Int(s)) = view.props.get("slides") {
            assert_eq!(*s, 5);
        } else {
            panic!("Expected Int prop 'slides'");
        }

        if let Some(PropValue::Bool(l)) = view.props.get("loop") {
            assert!(*l);
        } else {
            panic!("Expected Bool prop 'loop'");
        }
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaCarousel.name(), "rgui_components::WaCarousel");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(
            WaCarouselMessage::SlideChange.message_name(),
            "slide_change"
        );
        assert_eq!(WaCarouselMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaCarouselState::schema_name(), "WaCarouselState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaCarouselState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), std::any::TypeId::of::<WaCarouselState>());
    }
}
