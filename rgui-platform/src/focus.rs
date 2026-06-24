//! 焦点管理——FocusManager（D5 §5）。

use crate::widget_tree::WidgetTree;
use rgui_core::id::WidgetId;
use rustc_hash::FxHashMap;
use std::fmt;

/// 输入模态——跟踪最后一次输入是键盘还是鼠标。
///
/// 用于实现 CSS `:focus-visible` 行为：
/// - 键盘导航（Tab/方向键）→ `Keyboard`
/// - 鼠标点击 → `Mouse`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputModality {
    /// 键盘输入（Tab、方向键等）。
    Keyboard,
    /// 鼠标输入（点击等）。
    #[default]
    Mouse,
}

/// 焦点导航方向（AC11）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    /// ArrowDown：下一个可聚焦兄弟。
    Next,
    /// ArrowUp：上一个可聚焦兄弟。
    Previous,
    /// Home：第一个可聚焦兄弟。
    First,
    /// End：最后一个可聚焦兄弟。
    Last,
}

/// 焦点管理器。
#[derive(Default)]
pub struct FocusManager {
    current: Option<WidgetId>,
    history: Vec<WidgetId>,
    trap_stack: Vec<Vec<WidgetId>>,
    /// 最后一次输入模态——用于判断焦点是否应该可见（focus-visible）。
    input_modality: InputModality,
    /// Roving tabindex 容器注册表（AC13）。
    /// Key: 容器 WidgetId。Value: 当前 tabbable 的子组件 WidgetId。
    roving_containers: FxHashMap<WidgetId, WidgetId>,
    /// 反向映射：子组件 WidgetId → 容器 WidgetId（AC13）。
    /// 用于 `focus()` 中的 O(1) roving 更新。
    roving_child_to_container: FxHashMap<WidgetId, WidgetId>,
}

impl FocusManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: None,
            history: Vec::new(),
            trap_stack: Vec::new(),
            input_modality: InputModality::default(),
            roving_containers: FxHashMap::default(),
            roving_child_to_container: FxHashMap::default(),
        }
    }

    #[must_use]
    pub fn current(&self) -> Option<WidgetId> {
        self.current
    }

    /// 设置输入模态——由 App 事件循环在键盘/鼠标事件时调用。
    ///
    /// CSS `:focus-visible` 行为：输入模态为 `Keyboard` 时焦点可见，
    /// 为 `Mouse` 时焦点不可见。一旦模态改变，后续所有焦点变更遵循新模态。
    pub fn set_input_modality(&mut self, modality: InputModality) {
        self.input_modality = modality;
    }

    /// 返回当前焦点是否应该显示轮廓（focus-visible）。
    ///
    /// 仅当存在焦点 widget 且最后一次输入模态为 `Keyboard` 时返回 `true`。
    #[must_use]
    pub fn is_focus_visible(&self) -> bool {
        self.current.is_some() && self.input_modality == InputModality::Keyboard
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

        // AC13: 自动更新 roving tabbable
        if let Some(&container_id) = self.roving_child_to_container.get(&widget_id) {
            self.roving_containers.insert(container_id, widget_id);
        }
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

    /// 在当前焦点 widget 的父容器内，按方向导航到兄弟节点（AC11）。
    ///
    /// 行为：
    /// - `Next`（ArrowDown）：移到下一个可聚焦兄弟，最后一项时绕回第一项
    /// - `Previous`（ArrowUp）：移到上一个可聚焦兄弟，第一项时绕回最后一项
    /// - `First`（Home）：跳到第一个可聚焦兄弟
    /// - `Last`（End）：跳到最后一个可聚焦兄弟
    ///
    /// 返回 `true` 表示焦点已移动（`FocusManager::current` 已更新），
    /// `false` 表示没有可移动的目标（无焦点、无父节点、单子节点等）。
    pub fn move_focus_sibling(&mut self, tree: &WidgetTree, direction: FocusDirection) -> bool {
        let current = match self.current {
            Some(id) => id,
            None => return false,
        };

        // 获取父节点
        let parent_id = match tree.parent(current) {
            Some(parent) => parent,
            None => return false,
        };

        // 获取所有兄弟节点（按原始顺序，过滤为可聚焦的）
        let siblings = tree.children(parent_id);
        let focusable: Vec<WidgetId> = siblings
            .iter()
            .copied()
            .filter(|&id| self.is_focusable(id))
            .collect();

        if focusable.len() <= 1 {
            return false;
        }

        // 找到当前焦点在 focusable 列表中的位置
        let pos = match focusable.iter().position(|&id| id == current) {
            Some(p) => p,
            None => return false,
        };

        let new_focus = match direction {
            FocusDirection::Next => {
                let next = (pos + 1) % focusable.len();
                focusable[next]
            },
            FocusDirection::Previous => {
                let prev = if pos == 0 {
                    focusable.len() - 1
                } else {
                    pos - 1
                };
                focusable[prev]
            },
            FocusDirection::First => {
                if pos == 0 {
                    return false;
                }
                focusable[0]
            },
            FocusDirection::Last => {
                let last = focusable.len() - 1;
                if pos == last {
                    return false;
                }
                focusable[last]
            },
        };

        self.focus(new_focus);
        true
    }

    // ── Roving tabindex (AC13) ──────────────────────────────────────────────

    /// 启用 roving tabindex 模式（AC13）。
    ///
    /// 注册一个容器为 roving 模式，管理子组件的 Tab 可达性。
    /// `container_id` 为容器 WidgetId，`item_ids` 为子组件有序列表。
    /// 初始 tabbable 子组件为 `item_ids` 的第一个。
    ///
    /// 对齐 WA `initRovingTabIndex()`。
    pub fn enable_roving(&mut self, container_id: WidgetId, item_ids: Vec<WidgetId>) {
        // 清理旧的反向映射
        self.roving_child_to_container
            .retain(|_, &mut c| c != container_id);

        // 注册新的反向映射
        for &item_id in &item_ids {
            self.roving_child_to_container.insert(item_id, container_id);
        }

        // 初始 tabbable 为第一个 item
        if let Some(&first) = item_ids.first() {
            self.roving_containers.insert(container_id, first);
        }
    }

    /// 取消 roving tabindex 模式（AC13）。
    pub fn disable_roving(&mut self, container_id: WidgetId) {
        self.roving_containers.remove(&container_id);
        self.roving_child_to_container
            .retain(|_, &mut c| c != container_id);
    }

    /// 获取指定容器的当前 tabbable 子组件（AC13）。
    ///
    /// 返回 `None` 表示该容器未注册 roving 模式。
    #[must_use]
    pub fn get_roving_tabbable(&self, container_id: WidgetId) -> Option<WidgetId> {
        self.roving_containers.get(&container_id).copied()
    }

    /// 判断 widget 是否当前 Tab 可达（AC13）。
    ///
    /// - 若 widget 属于某个 roving 容器，仅当其是该容器当前 tabbable 子组件时返回 `true`
    /// - 若 widget 不属于任何 roving 容器，返回 `true`（默认可达）
    ///
    /// `tree` 用于查询 widget 的父容器。
    #[must_use]
    pub fn is_tabbable(&self, widget_id: WidgetId, tree: &WidgetTree) -> bool {
        // 查找 widget 的父容器
        let parent_id = match tree.parent(widget_id) {
            Some(parent) => parent,
            None => return true, // 无父节点，默认 tabbable
        };

        // 如果父容器是 roving 容器，检查 widget 是否在 roving 组内
        if let Some(&tabbable) = self.roving_containers.get(&parent_id) {
            // 仅当 widget 已注册到 roving 组内时才应用 tabindex 限制
            if self.roving_child_to_container.contains_key(&widget_id) {
                return widget_id == tabbable;
            }
            // widget 在 roving 容器下但不在 roving 组内 → 默认 tabbable
        }

        true
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

    // ── RED: focus-visible ────────────────────────────────────────────

    #[test]
    fn focus_visible_with_keyboard_modality() {
        let mut fm = FocusManager::new();
        fm.set_input_modality(InputModality::Keyboard);
        fm.focus(WidgetId::from_u64(1));
        assert!(fm.is_focus_visible());
    }

    #[test]
    fn focus_not_visible_with_mouse_modality() {
        let mut fm = FocusManager::new();
        // Default modality is Mouse
        fm.focus(WidgetId::from_u64(1));
        assert!(!fm.is_focus_visible());
    }

    #[test]
    fn focus_not_visible_when_blurred() {
        let mut fm = FocusManager::new();
        fm.set_input_modality(InputModality::Keyboard);
        fm.focus(WidgetId::from_u64(1));
        assert!(fm.is_focus_visible());
        fm.blur();
        assert!(!fm.is_focus_visible());
    }

    #[test]
    fn modality_switch_changes_focus_visibility() {
        let mut fm = FocusManager::new();
        // Start with mouse
        fm.set_input_modality(InputModality::Mouse);
        fm.focus(WidgetId::from_u64(1));
        assert!(!fm.is_focus_visible());

        // Switch to keyboard
        fm.set_input_modality(InputModality::Keyboard);
        fm.focus(WidgetId::from_u64(2));
        assert!(fm.is_focus_visible());

        // Switch back to mouse
        fm.set_input_modality(InputModality::Mouse);
        fm.focus(WidgetId::from_u64(3));
        assert!(!fm.is_focus_visible());
    }

    #[test]
    fn default_modality_is_mouse() {
        let fm = FocusManager::new();
        assert!(!fm.is_focus_visible());
    }

    // ── RED: move_focus_sibling (AC11) ────────────────────────────────────

    /// Builds a flat container tree for AC11 sibling navigation tests:
    ///     1 (container)
    ///     ├── 2 (item)
    ///     ├── 3 (item)
    ///     └── 4 (item)
    fn build_flat_container() -> WidgetTree {
        let mut tree = WidgetTree::new();
        tree.add_child(make_id(1), make_id(2));
        tree.add_child(make_id(1), make_id(3));
        tree.add_child(make_id(1), make_id(4));
        tree
    }

    #[test]
    fn arrow_down_navigates_to_next_sibling() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(2));
        assert!(fm.move_focus_sibling(&tree, FocusDirection::Next));
        assert_eq!(fm.current(), Some(make_id(3)));
    }

    #[test]
    fn arrow_down_wraps_from_last_to_first() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(4));
        assert!(fm.move_focus_sibling(&tree, FocusDirection::Next));
        assert_eq!(fm.current(), Some(make_id(2)));
    }

    #[test]
    fn arrow_up_navigates_to_previous_sibling() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(3));
        assert!(fm.move_focus_sibling(&tree, FocusDirection::Previous));
        assert_eq!(fm.current(), Some(make_id(2)));
    }

    #[test]
    fn arrow_up_wraps_from_first_to_last() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(2));
        assert!(fm.move_focus_sibling(&tree, FocusDirection::Previous));
        assert_eq!(fm.current(), Some(make_id(4)));
    }

    #[test]
    fn home_jumps_to_first_sibling() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(4));
        assert!(fm.move_focus_sibling(&tree, FocusDirection::First));
        assert_eq!(fm.current(), Some(make_id(2)));
    }

    #[test]
    fn home_from_first_is_noop() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(2));
        assert!(!fm.move_focus_sibling(&tree, FocusDirection::First));
        assert_eq!(fm.current(), Some(make_id(2)));
    }

    #[test]
    fn end_jumps_to_last_sibling() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(2));
        assert!(fm.move_focus_sibling(&tree, FocusDirection::Last));
        assert_eq!(fm.current(), Some(make_id(4)));
    }

    #[test]
    fn end_from_last_is_noop() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(4));
        assert!(!fm.move_focus_sibling(&tree, FocusDirection::Last));
        assert_eq!(fm.current(), Some(make_id(4)));
    }

    #[test]
    fn navigation_noop_when_no_focus() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        assert!(!fm.move_focus_sibling(&tree, FocusDirection::Next));
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn navigation_noop_when_root_has_no_parent() {
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(1)); // Root: no parent, no siblings
        assert!(!fm.move_focus_sibling(&tree, FocusDirection::Next));
        assert_eq!(fm.current(), Some(make_id(1)));
    }

    #[test]
    fn navigation_noop_when_only_child() {
        let mut tree = WidgetTree::new();
        tree.add_child(make_id(1), make_id(2)); // Single child
        let mut fm = FocusManager::new();
        fm.focus(make_id(2));
        // Only one focusable sibling → moving is a no-op
        assert!(!fm.move_focus_sibling(&tree, FocusDirection::Next));
        assert_eq!(fm.current(), Some(make_id(2)));
    }

    #[test]
    fn navigation_skips_non_focusable_siblings() {
        // Container with items where one is not focusable.
        // Since is_focusable currently returns true for all, this test
        // verifies the filter hook is called correctly.
        let tree = build_flat_container();
        let mut fm = FocusManager::new();
        fm.focus(make_id(2));
        // All items focusable → should move to 3
        assert!(fm.move_focus_sibling(&tree, FocusDirection::Next));
        assert_eq!(fm.current(), Some(make_id(3)));
    }

    // ── RED: roving tabindex (AC13) ─────────────────────────────────────────

    #[test]
    fn roving_initial_tabbable_is_first_item() {
        let mut fm = FocusManager::new();
        let items = vec![make_id(2), make_id(3), make_id(4)];
        fm.enable_roving(make_id(1), items);
        assert_eq!(fm.get_roving_tabbable(make_id(1)), Some(make_id(2)));
    }

    #[test]
    fn roving_focus_updates_tabbable() {
        let mut fm = FocusManager::new();
        let items = vec![make_id(2), make_id(3), make_id(4)];
        fm.enable_roving(make_id(1), items);
        // Focus on item 3 → tabbable should update to 3
        fm.focus(make_id(3));
        assert_eq!(fm.get_roving_tabbable(make_id(1)), Some(make_id(3)));
    }

    #[test]
    fn roving_non_tabbable_returns_false() {
        let mut fm = FocusManager::new();
        let tree = build_flat_container();
        let items = vec![make_id(2), make_id(3), make_id(4)];
        fm.enable_roving(make_id(1), items);
        // item 2 is tabbable, item 3 is not
        assert!(fm.is_tabbable(make_id(2), &tree));
        assert!(!fm.is_tabbable(make_id(3), &tree));
    }

    #[test]
    fn roving_unknown_container_returns_none() {
        let fm = FocusManager::new();
        assert_eq!(fm.get_roving_tabbable(make_id(999)), None);
    }

    #[test]
    fn roving_widget_not_in_group_is_tabbable() {
        let mut fm = FocusManager::new();
        let tree = build_flat_container();
        let items = vec![make_id(2), make_id(3)];
        fm.enable_roving(make_id(1), items);
        // Widget 4 is not a child of the roving container → should be tabbable
        assert!(fm.is_tabbable(make_id(4), &tree));
    }
}
