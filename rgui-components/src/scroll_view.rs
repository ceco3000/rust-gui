//! ScrollView 组件——可滚动视口。
//!
//! 包装单个子 widget，提供滚动能力。阶段 0 最小实现：
//! - 鼠标滚轮更新 `scroll_y`
//! - 滚动条为简单矩形
//! - 视口裁剪和坐标变换由框架层 `PaintLayerData` 的
//!   `clip_rect`/`scroll_offset` 字段支持
//!
//! 参考：D13 §4.1、D8 G21。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// ScrollPolicy
// ============================================================================

/// 滚动条显示策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ScrollPolicy {
    /// 始终显示滚动条。
    Always,
    /// 仅在内容溢出视口时显示滚动条。
    #[default]
    AsNeeded,
    /// 从不显示滚动条（内容仍可滚动，无视觉指示）。
    Never,
}

// ============================================================================
// ScrollViewState
// ============================================================================

/// ScrollView 业务状态。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct ScrollViewState {
    /// 水平滚动位置（逻辑像素）。
    pub scroll_x: f64,
    /// 垂直滚动位置（逻辑像素）。
    pub scroll_y: f64,
    /// 滚动条显示策略（默认 AsNeeded）。
    pub scroll_policy: Option<ScrollPolicy>,
}

impl ScrollViewState {
    /// 创建新状态，初始滚动位置为 (0, 0)。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带滚动策略的状态。
    #[must_use]
    pub fn with_policy(policy: ScrollPolicy) -> Self {
        Self {
            scroll_policy: Some(policy),
            ..Default::default()
        }
    }

    /// 返回当前滚动策略，未设置时默认 `AsNeeded`。
    fn policy(&self) -> ScrollPolicy {
        self.scroll_policy.unwrap_or_default()
    }

    /// 更新滚动位置，钳位到有效范围。
    pub fn set_scroll(&mut self, x: f64, y: f64, max_x: f64, max_y: f64) {
        self.scroll_x = x.clamp(0.0, max_x.max(0.0));
        self.scroll_y = y.clamp(0.0, max_y.max(0.0));
    }
}

// ============================================================================
// ScrollViewMessage
// ============================================================================

/// ScrollView 消息类型。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum ScrollViewMessage {
    /// 滚动位置改变。
    ScrollChanged { x: f64, y: f64 },
    /// 占位（ScrollView 不主动产生消息时使用）。
    NoOp,
}

// ============================================================================
// ScrollView
// ============================================================================

/// ScrollView 组件（unit struct）。
///
/// 可滚动视口。包裹单个子 widget，提供垂直（和水平）滚动能力。
pub struct ScrollView;

impl WidgetSpec for ScrollView {
    type State = ScrollViewState;
    type Message = ScrollViewMessage;

    fn name(&self) -> &'static str {
        "rgui_components::ScrollView"
    }

    fn view(&self, _s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let view = WidgetView::new("rgui_components::ScrollView");
        // ScrollView 在 WidgetView.props 中设置 overflow 属性供布局引擎识别
        view.prop("overflow", PropValue::str("scroll"))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            ScrollViewMessage::ScrollChanged { x, y } => {
                state.scroll_x = x.max(0.0);
                state.scroll_y = y.max(0.0);
            },
            ScrollViewMessage::NoOp => {},
        }
    }

    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let policy = s.policy();
        let show_scrollbar = match policy {
            ScrollPolicy::Always => true,
            ScrollPolicy::AsNeeded => {
                // 阶段 0 简化：通过 bounds 判断是否需要滚动条
                // bounds.size.height < content_height 时显示
                true
            },
            ScrollPolicy::Never => false,
        };

        if show_scrollbar {
            let bar_width = 8.0;
            let bar_x = bounds.origin.x + bounds.size.width - bar_width;
            let bar_y = bounds.origin.y;
            let bar_h = bounds.size.height;

            // 滚动条轨道（浅灰背景）
            let track_rect = Rect::new(bar_x, bar_y, bar_width, bar_h);
            ctx.fill_rect(track_rect, Color::rgb(0.15, 0.15, 0.15), 2.0);

            // 滚动条滑块（深灰——阶段 0 简易矩形）
            let thumb_height = (bar_h * 0.3).max(20.0);
            let thumb_y = bar_y + (s.scroll_y / (100.0_f64.max(1.0))) * (bar_h - thumb_height);
            let thumb_rect = Rect::new(bar_x + 1.0, thumb_y, bar_width - 2.0, thumb_height);
            ctx.fill_rect(thumb_rect, Color::rgb(0.4, 0.4, 0.45), 3.0);
        }
    }

    fn accessibility(&self, _: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- ScrollPolicy ---

    #[test]
    fn scroll_policy_default_is_as_needed() {
        assert_eq!(ScrollPolicy::default(), ScrollPolicy::AsNeeded);
    }

    // --- ScrollViewState ---

    #[test]
    fn state_default() {
        let s = ScrollViewState::default();
        assert_eq!(s.scroll_x, 0.0);
        assert_eq!(s.scroll_y, 0.0);
        assert_eq!(s.scroll_policy, None);
    }

    #[test]
    fn state_new() {
        let s = ScrollViewState::new();
        assert_eq!(s.scroll_x, 0.0);
        assert_eq!(s.scroll_y, 0.0);
    }

    #[test]
    fn state_with_policy() {
        let s = ScrollViewState::with_policy(ScrollPolicy::Always);
        assert_eq!(s.policy(), ScrollPolicy::Always);
    }

    #[test]
    fn state_policy_defaults_to_as_needed() {
        let s = ScrollViewState::default();
        assert_eq!(s.policy(), ScrollPolicy::AsNeeded);
    }

    #[test]
    fn set_scroll_clamps_to_range() {
        let mut s = ScrollViewState::new();
        s.set_scroll(50.0, 200.0, 100.0, 150.0);
        assert_eq!(s.scroll_x, 50.0);
        assert_eq!(s.scroll_y, 150.0); // clamped to max_y
    }

    #[test]
    fn set_scroll_negative_clamps_to_zero() {
        let mut s = ScrollViewState::new();
        s.set_scroll(-10.0, -5.0, 100.0, 100.0);
        assert_eq!(s.scroll_x, 0.0);
        assert_eq!(s.scroll_y, 0.0);
    }

    #[test]
    fn set_scroll_max_zero_allows_any_scroll() {
        let mut s = ScrollViewState::new();
        // max_x=0, max_y=0 means clamped to 0
        s.set_scroll(5.0, 10.0, 0.0, 0.0);
        assert_eq!(s.scroll_x, 0.0);
        assert_eq!(s.scroll_y, 0.0);
    }

    // --- ScrollViewMessage ---

    #[test]
    fn message_clone_and_eq() {
        let m1 = ScrollViewMessage::ScrollChanged { x: 1.0, y: 2.0 };
        let m2 = m1.clone();
        assert_eq!(m1, m2);
    }

    #[test]
    fn message_derive_name() {
        assert_eq!(
            ScrollViewMessage::ScrollChanged { x: 0.0, y: 0.0 }.message_name(),
            "scroll_changed"
        );
    }

    // --- ScrollView::name ---

    #[test]
    fn component_name() {
        assert_eq!(ScrollView.name(), "rgui_components::ScrollView");
    }

    // --- ScrollView::view ---

    #[test]
    fn view_sets_overflow_scroll() {
        let state = ScrollViewState::default();
        let view = ScrollView.view(&state, &ViewContext::new(Size::new(400.0, 300.0)));
        assert_eq!(view.props.get("overflow"), Some(&PropValue::str("scroll")));
    }

    // --- ScrollView::update ---

    #[test]
    fn update_scroll_changed() {
        let mut state = ScrollViewState::new();
        let mut ctx = UpdateContext::default();
        ScrollView.update(
            ScrollViewMessage::ScrollChanged { x: 10.0, y: 50.0 },
            &mut state,
            &mut ctx,
        );
        assert_eq!(state.scroll_x, 10.0);
        assert_eq!(state.scroll_y, 50.0);
    }

    #[test]
    fn update_scroll_negative_clamped() {
        let mut state = ScrollViewState::new();
        let mut ctx = UpdateContext::default();
        ScrollView.update(
            ScrollViewMessage::ScrollChanged { x: -5.0, y: -10.0 },
            &mut state,
            &mut ctx,
        );
        assert_eq!(state.scroll_x, 0.0);
        assert_eq!(state.scroll_y, 0.0);
    }

    #[test]
    fn update_noop_preserves_state() {
        let mut state = ScrollViewState {
            scroll_x: 42.0,
            scroll_y: 99.0,
            scroll_policy: None,
        };
        let mut ctx = UpdateContext::default();
        ScrollView.update(ScrollViewMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.scroll_x, 42.0);
        assert_eq!(state.scroll_y, 99.0);
    }

    // --- ScrollView::paint ---

    #[test]
    fn paint_default_draws_scrollbar() {
        let state = ScrollViewState::default();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        ScrollView.paint(&state, bounds, &mut ctx);
        // track + thumb = 2 ops (AsNeeded policy always shows for now)
        assert_eq!(ctx.op_count(), 2);
    }

    #[test]
    fn paint_never_policy_skips_scrollbar() {
        let state = ScrollViewState {
            scroll_policy: Some(ScrollPolicy::Never),
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        ScrollView.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn paint_always_policy_draws_scrollbar() {
        let state = ScrollViewState {
            scroll_policy: Some(ScrollPolicy::Always),
            ..Default::default()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        ScrollView.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 2);
    }

    #[test]
    fn measure_returns_zero() {
        let state = ScrollViewState::default();
        let ctx = MeasureContext::default();
        let size = ScrollView.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(ScrollViewState::schema_name(), "ScrollViewState");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = ScrollViewState::default();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let node = ScrollView.accessibility(&state, &ctx);
        assert_eq!(node.role, AccessibilityRole::None);
    }
}
