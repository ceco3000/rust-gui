//! 脏区域追踪（D3 §9）。
//!
//! 记录本帧需要重绘的屏幕区域，支持合并重叠和相邻区域以减少冗余绘制。
//! 渲染后端可据此做增量提交。

use rgui_core::geometry::Rect;

/// 脏区域追踪器。
///
/// 记录本帧需要重绘的屏幕区域。渲染后端可据此做增量提交，
/// 避免全屏重绘。
///
/// # 合并策略
///
/// `coalesce()` 使用 X 轴排序的合并算法：按矩形左边界排序后，
/// 将重叠或在 `margin`（2px）范围内的相邻矩形合并为最小外接矩形。
/// 这减少了脏区域数量，同时不会显著增加重绘面积。
///
/// # 示例
///
/// ```
/// use rgui_core::geometry::Rect;
/// use rgui_render::dirty::DirtyRegionTracker;
///
/// let mut tracker = DirtyRegionTracker::new();
/// tracker.mark_dirty(Rect::new(0.0, 0.0, 100.0, 50.0));
/// tracker.mark_dirty(Rect::new(50.0, 25.0, 100.0, 50.0));
/// let regions = tracker.drain();
/// assert_eq!(regions.len(), 1); // 重叠区域已合并
/// ```
#[derive(Debug)]
pub struct DirtyRegionTracker {
    regions: Vec<Rect>,
}

impl DirtyRegionTracker {
    /// 创建新的脏区域追踪器。
    #[must_use]
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// 标记一个脏区域。
    ///
    /// 空矩形（宽度或高度 ≤ 0）将被忽略。
    pub fn mark_dirty(&mut self, rect: Rect) {
        if rect.is_empty() {
            return;
        }
        self.regions.push(rect);
    }

    /// 合并重叠和相邻（2px 范围内）的脏矩形。
    ///
    /// 使用 X 轴排序合并算法，减少脏矩形数量。
    /// 当脏矩形数量 ≤ 1 时不做任何操作。
    pub fn coalesce(&mut self) {
        if self.regions.len() <= 1 {
            return;
        }

        let mut sorted = std::mem::take(&mut self.regions);
        sorted.sort_unstable_by(|a, b| a.origin.x.total_cmp(&b.origin.x));

        const COALESCE_MARGIN: f64 = 2.0;

        let mut merged: Vec<Rect> = Vec::with_capacity(sorted.len());
        let mut current = sorted[0];

        for &rect in sorted.iter().skip(1) {
            if rects_near(&current, &rect, COALESCE_MARGIN) {
                current = merge_rects(current, rect);
            } else {
                merged.push(current);
                current = rect;
            }
        }
        merged.push(current);

        self.regions = merged;
    }

    /// 取出所有脏区域并清空（自动合并重叠区域）。
    ///
    /// 在返回前自动执行 `coalesce()`，确保返回的脏矩形列表已合并。
    #[must_use]
    pub fn drain(&mut self) -> Vec<Rect> {
        self.coalesce();
        std::mem::take(&mut self.regions)
    }

    /// 返回当前脏区域列表（不执行合并）。
    #[must_use]
    pub fn regions(&self) -> &[Rect] {
        &self.regions
    }

    /// 是否存在脏区域。
    #[must_use]
    pub fn has_dirty(&self) -> bool {
        !self.regions.is_empty()
    }

    /// 清空所有脏区域。
    pub fn clear(&mut self) {
        self.regions.clear();
    }
}

impl Default for DirtyRegionTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// 判断两个矩形是否接近（在 `margin` 像素内视为可合并）。
///
/// 检查两个矩形在水平或垂直方向上的间距是否 ≤ `margin`。
/// 重叠的矩形始终被视为接近。
#[must_use]
fn rects_near(a: &Rect, b: &Rect, margin: f64) -> bool {
    let ax2 = a.right();
    let ay2 = a.bottom();
    let bx2 = b.right();
    let by2 = b.bottom();

    !(ax2 + margin < b.origin.x
        || bx2 + margin < a.origin.x
        || ay2 + margin < b.origin.y
        || by2 + margin < a.origin.y)
}

/// 合并两个矩形为最小外接矩形。
#[must_use]
fn merge_rects(a: Rect, b: Rect) -> Rect {
    let x = a.origin.x.min(b.origin.x);
    let y = a.origin.y.min(b.origin.y);
    let x2 = a.right().max(b.right());
    let y2 = a.bottom().max(b.bottom());
    Rect::from_ltrb(x, y, x2, y2)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- 基础功能 ---

    #[test]
    fn new_is_empty() {
        let tracker = DirtyRegionTracker::new();
        assert!(!tracker.has_dirty());
        assert!(tracker.regions().is_empty());
    }

    #[test]
    fn default_is_empty() {
        let tracker = DirtyRegionTracker::default();
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn mark_dirty_adds_region() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(10.0, 20.0, 100.0, 50.0));
        assert!(tracker.has_dirty());
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(10.0, 20.0, 100.0, 50.0));
    }

    #[test]
    fn mark_dirty_ignores_empty_rect() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::ZERO);
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn mark_dirty_ignores_negative_size() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::from_ltrb(10.0, 10.0, 5.0, 5.0));
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn mark_dirty_multiple_regions() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 50.0, 50.0));
        tracker.mark_dirty(Rect::new(100.0, 100.0, 50.0, 50.0));
        tracker.mark_dirty(Rect::new(200.0, 200.0, 50.0, 50.0));
        assert_eq!(tracker.regions().len(), 3);
    }

    #[test]
    fn clear_removes_all() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(tracker.has_dirty());
        tracker.clear();
        assert!(!tracker.has_dirty());
        assert!(tracker.regions().is_empty());
    }

    // --- coalesce ---

    #[test]
    fn coalesce_merges_overlapping() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(5.0, 5.0, 10.0, 10.0));
        tracker.coalesce();
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(0.0, 0.0, 15.0, 15.0));
    }

    #[test]
    fn coalesce_merges_adjacent_within_margin() {
        let mut tracker = DirtyRegionTracker::new();
        // 1px gap < 2px margin → 合并
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(11.0, 0.0, 10.0, 10.0));
        tracker.coalesce();
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(0.0, 0.0, 21.0, 10.0));
    }

    #[test]
    fn coalesce_keeps_disjoint_regions() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(50.0, 50.0, 10.0, 10.0));
        tracker.coalesce();
        assert_eq!(tracker.regions().len(), 2);
    }

    #[test]
    fn coalesce_disjoint_keeps_original_rects() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(50.0, 50.0, 10.0, 10.0));
        tracker.coalesce();
        assert!(tracker.regions().contains(&Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert!(
            tracker
                .regions()
                .contains(&Rect::new(50.0, 50.0, 10.0, 10.0))
        );
    }

    #[test]
    fn coalesce_single_region_is_noop() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(10.0, 20.0, 30.0, 40.0));
        tracker.coalesce();
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn coalesce_empty_tracker_is_noop() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.coalesce();
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn coalesce_merges_chain_of_overlapping() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(8.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(16.0, 0.0, 10.0, 10.0));
        tracker.coalesce();
        // 0,0-10,10 和 8,0-18,10 合并为 0,0-18,10
        // 0,0-18,10 和 16,0-26,10 合并为 0,0-26,10
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(0.0, 0.0, 26.0, 10.0));
    }

    #[test]
    fn coalesce_sorts_by_x_before_merging() {
        let mut tracker = DirtyRegionTracker::new();
        // 乱序添加
        tracker.mark_dirty(Rect::new(50.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(25.0, 0.0, 10.0, 10.0));
        tracker.coalesce();
        // 排序后: 0,0-10,10 (gap to 25,0-35,0: 15 > 2) → 不合并
        // 25,0-35,0 (gap to 50,0-60,0: 15 > 2) → 不合并
        assert_eq!(tracker.regions().len(), 3);
    }

    #[test]
    fn coalesce_merges_contained_rect() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 100.0, 100.0));
        tracker.mark_dirty(Rect::new(10.0, 10.0, 20.0, 20.0));
        tracker.coalesce();
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn coalesce_merges_vertical_overlap() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 50.0, 10.0));
        tracker.mark_dirty(Rect::new(0.0, 8.0, 50.0, 10.0));
        tracker.coalesce();
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(0.0, 0.0, 50.0, 18.0));
    }

    // --- drain ---

    #[test]
    fn drain_returns_all_and_clears() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(20.0, 20.0, 10.0, 10.0));
        let drained = tracker.drain();
        assert_eq!(drained.len(), 2);
        assert!(!tracker.has_dirty());
        assert!(tracker.regions().is_empty());
    }

    #[test]
    fn drain_coalesces_overlapping_regions() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(5.0, 5.0, 10.0, 10.0));
        let drained = tracker.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0], Rect::new(0.0, 0.0, 15.0, 15.0));
    }

    #[test]
    fn drain_empty_tracker() {
        let mut tracker = DirtyRegionTracker::new();
        let drained = tracker.drain();
        assert!(drained.is_empty());
        assert!(!tracker.has_dirty());
    }

    #[test]
    fn drain_twice_after_mark() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        let first = tracker.drain();
        assert_eq!(first.len(), 1);
        let second = tracker.drain();
        assert!(second.is_empty());

        // 重新标记
        tracker.mark_dirty(Rect::new(100.0, 100.0, 10.0, 10.0));
        assert!(tracker.has_dirty());
    }

    // --- 辅助函数 ---

    #[test]
    fn rects_near_overlapping() {
        assert!(rects_near(
            &Rect::new(0.0, 0.0, 10.0, 10.0),
            &Rect::new(5.0, 5.0, 10.0, 10.0),
            2.0,
        ));
    }

    #[test]
    fn rects_near_touching() {
        // 正好接触
        assert!(rects_near(
            &Rect::new(0.0, 0.0, 10.0, 10.0),
            &Rect::new(10.0, 0.0, 10.0, 10.0),
            2.0,
        ));
    }

    #[test]
    fn rects_near_within_margin() {
        // 2px gap = margin
        assert!(rects_near(
            &Rect::new(0.0, 0.0, 10.0, 10.0),
            &Rect::new(12.0, 0.0, 10.0, 10.0),
            2.0,
        ));
    }

    #[test]
    fn rects_near_beyond_margin() {
        // 3px gap > 2px margin
        assert!(!rects_near(
            &Rect::new(0.0, 0.0, 10.0, 10.0),
            &Rect::new(13.0, 0.0, 10.0, 10.0),
            2.0,
        ));
    }

    #[test]
    fn rects_near_vertical_overlap() {
        assert!(rects_near(
            &Rect::new(0.0, 0.0, 50.0, 10.0),
            &Rect::new(0.0, 8.0, 50.0, 10.0),
            2.0,
        ));
    }

    #[test]
    fn rects_near_far_apart() {
        assert!(!rects_near(
            &Rect::new(0.0, 0.0, 10.0, 10.0),
            &Rect::new(50.0, 0.0, 10.0, 10.0),
            2.0,
        ));
    }

    #[test]
    fn merge_rects_overlapping() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        let merged = merge_rects(a, b);
        assert_eq!(merged, Rect::new(0.0, 0.0, 15.0, 15.0));
    }

    #[test]
    fn merge_rects_disjoint() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(20.0, 20.0, 10.0, 10.0);
        let merged = merge_rects(a, b);
        assert_eq!(merged, Rect::from_ltrb(0.0, 0.0, 30.0, 30.0));
    }

    #[test]
    fn merge_rects_contained() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(10.0, 10.0, 20.0, 20.0);
        let merged = merge_rects(a, b);
        assert_eq!(merged, Rect::new(0.0, 0.0, 100.0, 100.0));
    }

    // --- 不变式 ---

    #[test]
    fn coalesce_idempotent() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        tracker.mark_dirty(Rect::new(5.0, 5.0, 15.0, 15.0));
        tracker.mark_dirty(Rect::new(50.0, 50.0, 10.0, 10.0));
        tracker.coalesce();
        let regions_after_first = tracker.regions().to_vec();
        // 二次合并应无变化
        tracker.coalesce();
        assert_eq!(tracker.regions(), regions_after_first.as_slice());
    }

    #[test]
    fn drain_leaves_tracker_in_clean_state() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        let _ = tracker.drain();
        // drain 后可以立即添加新区域
        tracker.mark_dirty(Rect::new(100.0, 100.0, 50.0, 50.0));
        assert_eq!(tracker.regions().len(), 1);
        assert_eq!(tracker.regions()[0], Rect::new(100.0, 100.0, 50.0, 50.0));
    }

    #[test]
    fn regions_returned_are_immutable() {
        let mut tracker = DirtyRegionTracker::new();
        tracker.mark_dirty(Rect::new(0.0, 0.0, 10.0, 10.0));
        let _ = tracker.regions();
        // 通过 regions() 获取切片后，仍可继续添加
        tracker.mark_dirty(Rect::new(20.0, 20.0, 10.0, 10.0));
        assert_eq!(tracker.regions().len(), 2);
    }
}
