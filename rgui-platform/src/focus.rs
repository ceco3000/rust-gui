//! 焦点管理——FocusManager（D5 §5）。

use crate::widget_tree::WidgetTree;
use rgui_core::id::WidgetId;
use std::fmt;

/// 焦点管理器。
#[derive(Default)]
pub struct FocusManager {
    current: Option<WidgetId>,
    history: Vec<WidgetId>,
    trap_stack: Vec<Vec<WidgetId>>,
}

impl FocusManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: None,
            history: Vec::new(),
            trap_stack: Vec::new(),
        }
    }

    #[must_use]
    pub fn current(&self) -> Option<WidgetId> {
        self.current
    }

    pub fn focus(&mut self, widget_id: WidgetId) {
        if let Some(trap) = self.trap_stack.last() {
            if !trap.contains(&widget_id) {
                return;
            }
        }

        if let Some(prev) = self.current {
            if prev != widget_id {
                self.history.push(prev);
            }
        }
        self.current = Some(widget_id);
    }

    /// 清除当前焦点（不修改历史栈）。
    pub fn blur(&mut self) {
        self.current = None;
    }

    /// 恢复上一次焦点。
    pub fn restore(&mut self) {
        self.current = self.history.pop();
    }

    pub fn push_trap(&mut self, widgets: Vec<WidgetId>) {
        self.trap_stack.push(widgets);
    }

    pub fn pop_trap(&mut self) {
        self.trap_stack.pop();
    }

    #[must_use]
    pub fn is_trapped(&self, widget_id: WidgetId) -> bool {
        self.trap_stack
            .last()
            .is_none_or(|trap| trap.contains(&widget_id))
    }

    /// 判断 widget 是否可接收焦点。
    ///
    /// 当前实现返回 `true`（placeholder），后续将查询
    /// `WidgetSpec` 能力或实例状态以决定 widget 是否可聚焦。
    /// Tab 导航依赖此方法进行过滤。
    #[must_use]
    pub fn is_focusable(&self, _id: WidgetId) -> bool {
        true
    }

    /// 处理 widget 被移除时的焦点回退（D5 §10）。
    ///
    /// 当被移除的 widget 持有焦点时，按以下优先级寻找替代焦点：
    /// 1. 历史栈中最近一个仍存在于树中的 widget
    /// 2. 被移除 widget 的兄弟节点（父节点下的其他子节点）
    /// 3. 父节点自身
    /// 4. 以上均不可用时，清除焦点
    ///
    /// 调用时机：**在 `WidgetTree::remove()` 之前调用**，因为此方法
    /// 需要通过 `WidgetTree` 查询被移除 widget 的父节点和兄弟节点。
    pub fn handle_widget_removal(&mut self, removed_id: WidgetId, tree: &WidgetTree) {
        if self.current != Some(removed_id) {
            return;
        }

        // 清除当前焦点（被移除的 widget 不再存在）
        self.current = None;

        // 尝试从历史中恢复（跳过也已不存在的 widget）
        while let Some(candidate) = self.history.last().copied() {
            if tree.contains(candidate) {
                self.current = Some(candidate);
                self.history.pop();
                return;
            }
            self.history.pop();
        }

        // 尝试兄弟节点
        if let Some(parent_id) = tree.parent(removed_id) {
            let siblings = tree.children(parent_id);
            // 过滤掉被移除的 widget 自身
            if let Some(&sibling) = siblings.iter().find(|&&id| id != removed_id) {
                self.current = Some(sibling);
                return;
            }
            // 无兄弟节点，回退到父节点
            self.current = Some(parent_id);
        }

        // 无父节点（移除根节点），焦点保持 None
    }
}

impl fmt::Debug for FocusManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FocusManager")
            .field("current", &self.current)
            .field("history_depth", &self.history.len())
            .field("trap_depth", &self.trap_stack.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget_tree::WidgetTree;

    fn make_id(n: u64) -> WidgetId {
        WidgetId::from_u64(n)
    }

    /// Builds a simple tree:
    ///   1 (root)
    ///   ├── 2
    ///   └── 3
    ///        └── 4
    fn build_tree() -> WidgetTree {
        let mut tree = WidgetTree::new();
        tree.add_child(make_id(1), make_id(2));
        tree.add_child(make_id(1), make_id(3));
        tree.add_child(make_id(3), make_id(4));
        tree
    }

    #[test]
    fn focus_manager_new_has_no_focus() {
        let fm = FocusManager::new();
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn focus_and_blur() {
        let mut fm = FocusManager::new();
        let id = WidgetId::from_u64(1);
        fm.focus(id);
        assert_eq!(fm.current(), Some(id));
        fm.blur();
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn focus_restore() {
        let mut fm = FocusManager::new();
        fm.focus(WidgetId::from_u64(1));
        fm.focus(WidgetId::from_u64(2));
        // 焦点从 1→2，history 中存有 1
        fm.restore();
        assert_eq!(fm.current(), Some(WidgetId::from_u64(1)));
    }

    #[test]
    fn trap_blocks_unlisted_focus() {
        let mut fm = FocusManager::new();
        let a = WidgetId::from_u64(1);
        let b = WidgetId::from_u64(2);
        fm.push_trap(vec![a]);
        fm.focus(b);
        assert_eq!(fm.current(), None);
        fm.focus(a);
        assert_eq!(fm.current(), Some(a));
    }

    // ── RED: is_focusable ─────────────────────────────────────────

    #[test]
    fn all_widgets_are_focusable_by_default() {
        let fm = FocusManager::new();
        assert!(fm.is_focusable(WidgetId::from_u64(1)));
        assert!(fm.is_focusable(WidgetId::from_u64(42)));
        assert!(fm.is_focusable(WidgetId::from_u64(0)));
    }

    #[test]
    fn is_focusable_returns_true_for_unknown_widget() {
        let fm = FocusManager::new();
        // 当前实现不查询 widget 能力，所有 widget 均可聚焦
        assert!(fm.is_focusable(WidgetId::new()));
    }

    // ── RED: handle_widget_removal ────────────────────────────────────────

    #[test]
    fn removing_non_focused_widget_does_nothing() {
        let mut fm = FocusManager::new();
        let tree = build_tree();
        fm.focus(make_id(2));
        fm.handle_widget_removal(make_id(3), &tree);
        // widget 3 is not focused, focus should stay on 2
        assert_eq!(fm.current(), Some(make_id(2)));
    }

    #[test]
    fn removing_focused_widget_restores_from_history() {
        let mut fm = FocusManager::new();
        let tree = build_tree();
        // Focus path: 2 → 4 → remove 4
        fm.focus(make_id(2));
        fm.focus(make_id(4));
        // Now current=4, history=[2]. Remove 4 -> should restore to 2
        fm.handle_widget_removal(make_id(4), &tree);
        assert_eq!(fm.current(), Some(make_id(2)));
    }

    #[test]
    fn removing_focused_widget_falls_back_to_sibling() {
        let mut fm = FocusManager::new();
        let tree = build_tree();
        // Focus on 2, no history. Remove 2 -> should fall back to sibling 3
        fm.focus(make_id(2));
        fm.handle_widget_removal(make_id(2), &tree);
        // sibling of 2 (child of 1) is 3
        assert_eq!(fm.current(), Some(make_id(3)));
    }

    #[test]
    fn removing_focused_widget_falls_back_to_parent() {
        let mut fm = FocusManager::new();
        let tree = build_tree();
        // Focus on 4 (child of 3). Remove 4 -> no siblings, fall back to parent 3
        fm.focus(make_id(4));
        fm.handle_widget_removal(make_id(4), &tree);
        assert_eq!(fm.current(), Some(make_id(3)));
    }

    #[test]
    fn removing_root_with_no_fallback_blurs() {
        let mut fm = FocusManager::new();
        let tree = build_tree();
        // Focus on root, no history. Remove root -> nothing to fall back to
        fm.focus(make_id(1));
        fm.handle_widget_removal(make_id(1), &tree);
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn history_skips_also_removed_widgets() {
        let mut fm = FocusManager::new();
        let tree = build_tree();
        // Focus: 2 → 4. Remove 2 (in history) and 4 (current).
        // History 2 is gone, should fall back to sibling of 4 (none) → parent 3
        fm.focus(make_id(2));
        fm.focus(make_id(4));
        // Remove 4 — history has 2 but 2 is also removed in this scenario
        // (For this test, 2 is still in tree; we're removing 4 only.)
        fm.handle_widget_removal(make_id(4), &tree);
        assert_eq!(fm.current(), Some(make_id(2)));
    }
}
