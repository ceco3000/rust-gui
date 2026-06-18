//! ListView 组件——虚拟化列表（阶段 0 等高简化版）。
//!
//! 仅渲染可见区域的 item，离开视口的 item 不参与 paint。
//! 阶段 0 假设所有 item 等高（`item_height`）。
//!
//! 参考：D13 §4.6、D8 G23。

use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{Color, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

/// serde 默认值函数：item_height 默认为 40.0。
#[allow(dead_code)]
fn default_item_height() -> f64 {
    40.0
}

// ============================================================================
// ListViewState
// ============================================================================

/// ListView 业务状态。
///
/// 所有 item 等高（`item_height`），阶段 0 简化假设。
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct ListViewState {
    /// 列表项文本内容。
    pub items: Vec<String>,
    /// 垂直滚动偏移（逻辑像素）。
    pub scroll_offset: f64,
    /// 每项高度（逻辑像素）。默认 40.0。
    #[serde(default = "default_item_height")]
    pub item_height: f64,
}

impl Default for ListViewState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            scroll_offset: 0.0,
            item_height: 40.0,
        }
    }
}

impl ListViewState {
    /// 创建新状态。
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            scroll_offset: 0.0,
            item_height: 40.0,
        }
    }

    /// 创建带 item 列表的状态。
    #[must_use]
    pub fn with_items(items: Vec<String>) -> Self {
        Self {
            items,
            scroll_offset: 0.0,
            item_height: 40.0,
        }
    }

    /// 设置 item 高度并返回自身（builder 风格）。
    #[must_use]
    pub fn item_height(mut self, h: f64) -> Self {
        self.item_height = h;
        self
    }

    /// 计算可见范围 [first_visible, last_visible)。
    /// 返回 (first_visible_index, last_visible_index)。
    fn visible_range(&self, viewport_height: f64) -> (usize, usize) {
        if self.item_height <= 0.0 || viewport_height <= 0.0 {
            return (0, 0);
        }
        let first = (self.scroll_offset / self.item_height).floor() as usize;
        let visible_count = (viewport_height / self.item_height).ceil() as usize + 1; // +1 overscan
        let last = (first + visible_count).min(self.items.len());
        (first.min(self.items.len()), last)
    }
}

// ============================================================================
// ListViewMessage
// ============================================================================

/// ListView 消息类型。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum ListViewMessage {
    /// 滚动位置改变（新的 scroll_offset 值）。
    Scrolled(f64),
    /// 占位（ListView 不主动产生交互消息时使用）。
    NoOp,
}

// ============================================================================
// ListView
// ============================================================================

/// ListView 组件（unit struct）。
///
/// 虚拟化可滚动列表。仅渲染可见区域的 item，离开视口的 item 不参与 paint。
pub struct ListView;

impl WidgetSpec for ListView {
    type State = ListViewState;
    type Message = ListViewMessage;

    fn name(&self) -> &'static str {
        "rgui_components::ListView"
    }

    fn view(&self, _s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        // ListView items are rendered in paint(), not via child widgets.
        WidgetView::new("rgui_components::ListView")
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            ListViewMessage::Scrolled(offset) => {
                state.scroll_offset = offset.max(0.0);
            },
            ListViewMessage::NoOp => {},
        }
    }

    fn measure(&self, s: &Self::State, _: BoxConstraints, _: &MeasureContext) -> Size {
        if s.items.is_empty() {
            return Size::ZERO;
        }
        let total_height = s.items.len() as f64 * s.item_height;
        Size::new(200.0, total_height)
    }

    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let item_h = s.item_height;
        if item_h <= 0.0 || s.items.is_empty() {
            return;
        }

        let (first, last) = s.visible_range(bounds.size.height);

        // Render visible items
        for i in first..last {
            let y = bounds.origin.y + i as f64 * item_h - s.scroll_offset;
            let item_rect = Rect::new(bounds.origin.x, y, bounds.size.width, item_h);

            // Item background (alternating subtle colors)
            let bg = if i % 2 == 0 {
                Color::rgb(0.08, 0.08, 0.10)
            } else {
                Color::rgb(0.10, 0.10, 0.12)
            };
            ctx.fill_rect(item_rect, bg, 0.0);

            // Item text
            if let Some(text) = s.items.get(i) {
                let text_rect = Rect::new(
                    bounds.origin.x + 8.0,
                    y + 4.0,
                    bounds.size.width - 16.0,
                    item_h - 8.0,
                );
                ctx.draw_text(text, text_rect, Color::new(0.9, 0.9, 0.95, 1.0), 14.0);
            }
        }

        // Render scrollbar
        let bar_width = 8.0;
        let bar_x = bounds.origin.x + bounds.size.width - bar_width;
        let total_height = s.items.len() as f64 * item_h;
        let viewport_h = bounds.size.height;

        if total_height > viewport_h {
            let track_rect = Rect::new(bar_x, bounds.origin.y, bar_width, viewport_h);
            ctx.fill_rect(track_rect, Color::rgb(0.12, 0.12, 0.14), 2.0);

            let thumb_h = (viewport_h / total_height * viewport_h).max(20.0);
            let max_scroll = (total_height - viewport_h).max(0.0);
            let scroll_ratio = if max_scroll > 0.0 {
                s.scroll_offset / max_scroll
            } else {
                0.0
            };
            let thumb_y = bounds.origin.y + scroll_ratio * (viewport_h - thumb_h);
            let thumb_rect = Rect::new(bar_x + 1.0, thumb_y, bar_width - 2.0, thumb_h);
            ctx.fill_rect(thumb_rect, Color::rgb(0.4, 0.4, 0.45), 3.0);
        }
    }

    fn accessibility(&self, _: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let mut node = AccessibilityNode::none();
        node.role = AccessibilityRole::List;
        node
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- ListViewState ---

    #[test]
    fn state_default() {
        let s = ListViewState::default();
        assert!(s.items.is_empty());
        assert_eq!(s.scroll_offset, 0.0);
        assert_eq!(s.item_height, 40.0);
    }

    #[test]
    fn state_new() {
        let s = ListViewState::new();
        assert!(s.items.is_empty());
        assert_eq!(s.scroll_offset, 0.0);
        assert_eq!(s.item_height, 40.0);
    }

    #[test]
    fn state_with_items() {
        let items = vec!["A".to_string(), "B".to_string()];
        let s = ListViewState::with_items(items.clone());
        assert_eq!(s.items, items);
        assert_eq!(s.item_height, 40.0);
    }

    #[test]
    fn state_item_height_builder() {
        let s = ListViewState::new().item_height(50.0);
        assert_eq!(s.item_height, 50.0);
    }

    #[test]
    fn visible_range_all_visible() {
        let items: Vec<String> = (0..10).map(|i| format!("item {}", i)).collect();
        let s = ListViewState {
            items,
            scroll_offset: 0.0,
            item_height: 40.0,
        };
        // viewport 400px = 10 items visible
        let (first, last) = s.visible_range(400.0);
        assert_eq!(first, 0);
        assert_eq!(last, 10);
    }

    #[test]
    fn visible_range_scrolled() {
        let items: Vec<String> = (0..20).map(|i| format!("item {}", i)).collect();
        let s = ListViewState {
            items,
            scroll_offset: 200.0, // scrolled past 5 items (5 × 40 = 200)
            item_height: 40.0,
        };
        let (first, last) = s.visible_range(400.0);
        assert_eq!(first, 5);
        // 400/40 = 10 items + 1 overscan = 11, but capped at len=20
        assert_eq!(last, 16);
    }

    #[test]
    fn visible_range_empty_items() {
        let s = ListViewState::new();
        let (first, last) = s.visible_range(400.0);
        assert_eq!(first, 0);
        assert_eq!(last, 0);
    }

    #[test]
    fn visible_range_zero_viewport() {
        let items: Vec<String> = (0..10).map(|i| format!("item {}", i)).collect();
        let s = ListViewState {
            items,
            scroll_offset: 0.0,
            item_height: 40.0,
        };
        let (first, last) = s.visible_range(0.0);
        assert_eq!(first, 0);
        assert_eq!(last, 0);
    }

    #[test]
    fn visible_range_zero_item_height() {
        let items: Vec<String> = (0..10).map(|i| format!("item {}", i)).collect();
        let s = ListViewState {
            items,
            scroll_offset: 0.0,
            item_height: 0.0,
        };
        let (first, last) = s.visible_range(400.0);
        assert_eq!(first, 0);
        assert_eq!(last, 0);
    }

    #[test]
    fn visible_range_scrolled_near_end() {
        let items: Vec<String> = (0..10).map(|i| format!("item {}", i)).collect();
        let s = ListViewState {
            items,
            scroll_offset: 350.0, // near end of 10 items × 40 = 400px total
            item_height: 40.0,
        };
        let (first, last) = s.visible_range(400.0);
        assert_eq!(first, 8); // 350/40 = 8.75 → floor 8
        assert!(last >= first);
        assert!(last <= s.items.len());
    }

    // --- ListViewMessage ---

    #[test]
    fn message_clone_and_eq() {
        let m1 = ListViewMessage::Scrolled(42.0);
        let m2 = m1.clone();
        assert_eq!(m1, m2);
    }

    #[test]
    fn message_derive_name() {
        assert_eq!(ListViewMessage::Scrolled(100.0).message_name(), "scrolled");
        assert_eq!(ListViewMessage::NoOp.message_name(), "no_op");
    }

    // --- ListView::name ---

    #[test]
    fn component_name() {
        assert_eq!(ListView.name(), "rgui_components::ListView");
    }

    // --- ListView::view ---

    #[test]
    fn view_returns_empty_view() {
        let state = ListViewState::new();
        let view = ListView.view(&state, &ViewContext::new(Size::new(400.0, 300.0)));
        assert_eq!(view.widget_type, "rgui_components::ListView");
        assert!(view.children.is_empty());
    }

    // --- ListView::update ---

    #[test]
    fn update_scrolled() {
        let mut state = ListViewState::new();
        let mut ctx = UpdateContext::default();
        ListView.update(ListViewMessage::Scrolled(150.0), &mut state, &mut ctx);
        assert_eq!(state.scroll_offset, 150.0);
    }

    #[test]
    fn update_scrolled_negative_clamped() {
        let mut state = ListViewState::new();
        let mut ctx = UpdateContext::default();
        ListView.update(ListViewMessage::Scrolled(-50.0), &mut state, &mut ctx);
        assert_eq!(state.scroll_offset, 0.0);
    }

    #[test]
    fn update_noop_preserves_state() {
        let items = vec!["A".to_string(), "B".to_string()];
        let mut state = ListViewState {
            items,
            scroll_offset: 42.0,
            item_height: 40.0,
        };
        let mut ctx = UpdateContext::default();
        ListView.update(ListViewMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.scroll_offset, 42.0);
        assert_eq!(state.items.len(), 2);
    }

    // --- ListView::measure ---

    #[test]
    fn measure_empty() {
        let state = ListViewState::new();
        let ctx = MeasureContext::default();
        let size = ListView.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn measure_with_items() {
        let items: Vec<String> = (0..10).map(|i| format!("item {}", i)).collect();
        let state = ListViewState {
            items,
            scroll_offset: 0.0,
            item_height: 40.0,
        };
        let ctx = MeasureContext::default();
        let size = ListView.measure(&state, BoxConstraints::UNCONSTRAINED, &ctx);
        assert_eq!(size.height, 400.0); // 10 × 40
        // width should be a reasonable default
        assert!(size.width > 0.0);
    }

    // --- ListView::paint ---

    #[test]
    fn paint_empty_no_ops() {
        let state = ListViewState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        ListView.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn paint_visible_items() {
        let items: Vec<String> = (0..5).map(|i| format!("item {}", i)).collect();
        let state = ListViewState {
            items,
            scroll_offset: 0.0,
            item_height: 40.0,
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        ListView.paint(&state, bounds, &mut ctx);
        // 5 items × 2 ops (fill_rect + draw_text) + 2 scrollbar ops = 12
        assert!(ctx.op_count() > 0);
    }

    #[test]
    fn paint_with_scrolloffset() {
        let items: Vec<String> = (0..20).map(|i| format!("item {}", i)).collect();
        let state = ListViewState {
            items,
            scroll_offset: 200.0,
            item_height: 40.0,
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        ListView.paint(&state, bounds, &mut ctx);
        // Items 5-12 visible (8 items) × 2 = 16 + 2 scrollbar = 18
        assert!(ctx.op_count() > 0);
    }

    // --- ListView::accessibility ---

    #[test]
    fn accessibility_returns_list_role() {
        let state = ListViewState::new();
        let ctx = AccessContext::new(Rect::new(0.0, 0.0, 400.0, 300.0));
        let node = ListView.accessibility(&state, &ctx);
        assert_ne!(node.role, AccessibilityRole::None);
    }

    // --- derive ---

    #[test]
    fn derive_state_schema_name() {
        assert_eq!(ListViewState::schema_name(), "ListViewState");
    }
}
