//! Widget 树——管理 widget 层级关系，提供遍历查询方法（D5 §3.1/§4.1）。
//!
//! ## 职责
//! - 维护 widget 父子关系
//! - 提供树遍历方法（path_to_root、traverse_visual_order）
//! - 提供根节点查询
//!
//! ## 消费者
//! - EventRouter：事件路由的捕获/冒泡阶段依赖 `path_to_root()`
//! - FocusManager：Tab 导航依赖 `traverse_visual_order()`
//! - AccessibilityTree：无障碍树构建依赖 `children()` / `traverse_visual_order()`

use rgui_core::id::WidgetId;
use rustc_hash::FxHashMap;

/// Widget 层级树。
///
/// 维护 widget 之间的父子关系，为事件路由、焦点管理和无障碍提供
/// 树遍历能力。
#[derive(Clone)]
pub struct WidgetTree {
    /// widget → 父 widget。
    parent: FxHashMap<WidgetId, WidgetId>,
    /// widget → 子 widget 列表（按视觉序/添加顺序）。
    children: FxHashMap<WidgetId, Vec<WidgetId>>,
}

impl WidgetTree {
    /// 创建空的 widget 树。
    #[must_use]
    pub fn new() -> Self {
        Self {
            parent: FxHashMap::default(),
            children: FxHashMap::default(),
        }
    }

    /// 添加父子关系。
    pub fn add_child(&mut self, parent_id: WidgetId, child_id: WidgetId) {
        self.parent.insert(child_id, parent_id);
        self.children.entry(parent_id).or_default().push(child_id);
    }

    /// 移除 widget 及其所有后代（级联删除）。
    pub fn remove(&mut self, widget_id: WidgetId) {
        // 级联删除子节点
        if let Some(kids) = self.children.remove(&widget_id) {
            for child in &kids {
                self.parent.remove(child);
                self.remove(*child);
            }
        }
        // 从父节点的 children 列表中移除自己
        if let Some(parent_id) = self.parent.remove(&widget_id) {
            if let Some(siblings) = self.children.get_mut(&parent_id) {
                siblings.retain(|&id| id != widget_id);
            }
        }
    }

    /// 查询 widget 的父节点。
    #[must_use]
    pub fn parent(&self, widget_id: WidgetId) -> Option<WidgetId> {
        self.parent.get(&widget_id).copied()
    }

    /// 查询 widget 的子节点列表。
    #[must_use]
    pub fn children(&self, widget_id: WidgetId) -> &[WidgetId] {
        self.children.get(&widget_id).map_or(&[], Vec::as_slice)
    }

    /// 判断 widget 是否在树中。
    #[must_use]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.parent.contains_key(&widget_id) || self.children.contains_key(&widget_id)
    }

    /// 获取从根到 widget 的路径（根在前，widget 在后）。
    ///
    /// 返回的 Vec 包含 widget 自身作为最后一个元素。
    #[must_use]
    pub fn path_to_root(&self, widget_id: WidgetId) -> Vec<WidgetId> {
        if !self.contains(widget_id) {
            return Vec::new();
        }
        let mut path = Vec::new();
        let mut current = Some(widget_id);
        while let Some(id) = current {
            path.push(id);
            current = self.parent.get(&id).copied();
        }
        path.reverse();
        path
    }

    /// 获取树的根节点。
    ///
    /// 根节点是没有父节点的顶层 widget。如果树中有多个孤立节点，
    /// 返回第一个找到的。空树返回 `None`。
    #[must_use]
    pub fn root(&self) -> Option<WidgetId> {
        // 从任意有 children 的节点出发，找到第一个没有父节点的
        self.children
            .keys()
            .find(|&&id| !self.parent.contains_key(&id))
            .copied()
            // 如果树只有单个节点（仅有父关系但无子节点），
            // 从 parent values 中找到第一个无父节点的
            .or_else(|| {
                self.parent
                    .values()
                    .find(|&&id| !self.parent.contains_key(&id))
                    .copied()
            })
    }

    /// DFS 前序遍历，产生视觉序列表。
    ///
    /// 从根节点开始，按 children 顺序深度优先遍历，
    /// 结果可用于 Tab 序重建（配合 FocusManager::is_focusable 过滤）。
    #[must_use]
    pub fn traverse_visual_order(&self) -> Vec<WidgetId> {
        let root = match self.root() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let mut result = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            result.push(id);
            // 子节点按逆序入栈以保持正向顺序（先添加的先被遍历）
            if let Some(kids) = self.children.get(&id) {
                for &child in kids.iter().rev() {
                    stack.push(child);
                }
            }
        }
        result
    }

    /// 获取 widget 的子节点数量。
    #[must_use]
    pub fn child_count(&self, widget_id: WidgetId) -> usize {
        self.children.get(&widget_id).map_or(0, Vec::len)
    }

    /// 树中的 widget 总数。
    #[must_use]
    pub fn len(&self) -> usize {
        // 节点数 = 有 parent 的节点 + 根节点（在 children keys 但不在 parent keys）
        let orphan_count = self
            .children
            .keys()
            .filter(|k| !self.parent.contains_key(k))
            .count();
        self.parent.len() + orphan_count
    }

    /// 树是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty() && self.parent.is_empty()
    }
}

impl Default for WidgetTree {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WidgetTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetTree")
            .field(
                "nodes",
                &self.parent.len().saturating_add(
                    self.children
                        .keys()
                        .filter(|k| !self.parent.contains_key(k))
                        .count(),
                ),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建一棵简单的三层树:
    ///     root (1)
    ///     ├── child_a (2)
    ///     │   ├── leaf_a1 (4)
    ///     │   └── leaf_a2 (5)
    ///     └── child_b (3)
    fn build_tree() -> WidgetTree {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(3));
        tree.add_child(WidgetId::from_u64(2), WidgetId::from_u64(4));
        tree.add_child(WidgetId::from_u64(2), WidgetId::from_u64(5));
        tree
    }

    // ── RED: path_to_root ────────────────────────────────────────────

    #[test]
    fn path_to_root_leaf_to_root() {
        let tree = build_tree();
        let path = tree.path_to_root(WidgetId::from_u64(5));
        assert_eq!(
            path,
            vec![
                WidgetId::from_u64(1),
                WidgetId::from_u64(2),
                WidgetId::from_u64(5),
            ]
        );
    }

    #[test]
    fn path_to_root_root_is_self() {
        let tree = build_tree();
        let path = tree.path_to_root(WidgetId::from_u64(1));
        assert_eq!(path, vec![WidgetId::from_u64(1)]);
    }

    #[test]
    fn path_to_root_unknown_returns_empty() {
        let tree = build_tree();
        let path = tree.path_to_root(WidgetId::from_u64(99));
        assert!(path.is_empty());
    }

    // ── RED: root ────────────────────────────────────────────────────

    #[test]
    fn root_returns_top_node() {
        let tree = build_tree();
        assert_eq!(tree.root(), Some(WidgetId::from_u64(1)));
    }

    #[test]
    fn root_empty_tree_returns_none() {
        let tree = WidgetTree::new();
        assert_eq!(tree.root(), None);
    }

    // ── RED: traverse_visual_order ───────────────────────────────────

    #[test]
    fn traverse_visual_order_dfs_preorder() {
        let tree = build_tree();
        let order = tree.traverse_visual_order();
        assert_eq!(
            order,
            vec![
                WidgetId::from_u64(1),
                WidgetId::from_u64(2),
                WidgetId::from_u64(4),
                WidgetId::from_u64(5),
                WidgetId::from_u64(3),
            ]
        );
    }

    #[test]
    fn traverse_visual_order_empty_tree() {
        let tree = WidgetTree::new();
        assert!(tree.traverse_visual_order().is_empty());
    }

    #[test]
    fn traverse_visual_order_single_node() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(10), WidgetId::from_u64(20));
        let order = tree.traverse_visual_order();
        assert_eq!(order, vec![WidgetId::from_u64(10), WidgetId::from_u64(20)]);
    }

    // ── RED: parent / children ───────────────────────────────────────

    #[test]
    fn parent_of_child() {
        let tree = build_tree();
        assert_eq!(
            tree.parent(WidgetId::from_u64(2)),
            Some(WidgetId::from_u64(1))
        );
    }

    #[test]
    fn parent_of_root_is_none() {
        let tree = build_tree();
        assert_eq!(tree.parent(WidgetId::from_u64(1)), None);
    }

    #[test]
    fn parent_of_unknown_is_none() {
        let tree = build_tree();
        assert_eq!(tree.parent(WidgetId::from_u64(99)), None);
    }

    #[test]
    fn children_of_node() {
        let tree = build_tree();
        let kids = tree.children(WidgetId::from_u64(2));
        assert_eq!(kids, &[WidgetId::from_u64(4), WidgetId::from_u64(5)]);
    }

    #[test]
    fn children_of_leaf_is_empty() {
        let tree = build_tree();
        assert!(tree.children(WidgetId::from_u64(4)).is_empty());
    }

    #[test]
    fn children_of_unknown_is_empty() {
        let tree = build_tree();
        assert!(tree.children(WidgetId::from_u64(99)).is_empty());
    }

    // ── RED: contains ────────────────────────────────────────────────

    #[test]
    fn contains_known_widget() {
        let tree = build_tree();
        assert!(tree.contains(WidgetId::from_u64(1)));
        assert!(tree.contains(WidgetId::from_u64(5)));
    }

    #[test]
    fn contains_unknown_widget() {
        let tree = build_tree();
        assert!(!tree.contains(WidgetId::from_u64(99)));
    }

    #[test]
    fn contains_empty_tree() {
        let tree = WidgetTree::new();
        assert!(!tree.contains(WidgetId::from_u64(1)));
    }

    // ── RED: remove cascade ──────────────────────────────────────────

    #[test]
    fn remove_cascades_to_children() {
        let mut tree = build_tree();
        tree.remove(WidgetId::from_u64(2));
        assert!(!tree.contains(WidgetId::from_u64(2)));
        assert!(!tree.contains(WidgetId::from_u64(4)));
        assert!(!tree.contains(WidgetId::from_u64(5)));
        // child_b (3) 仍然存在
        assert!(tree.contains(WidgetId::from_u64(3)));
        assert!(tree.contains(WidgetId::from_u64(1)));
    }

    #[test]
    fn remove_root_leaves_empty() {
        let mut tree = build_tree();
        tree.remove(WidgetId::from_u64(1));
        assert!(tree.is_empty());
    }

    // ── RED: add_child / len / is_empty ──────────────────────────────

    #[test]
    fn new_tree_is_empty() {
        let tree = WidgetTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn add_child_increases_count() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        assert_eq!(tree.len(), 2);
        assert!(!tree.is_empty());
    }

    #[test]
    fn child_count_of_node() {
        let tree = build_tree();
        assert_eq!(tree.child_count(WidgetId::from_u64(2)), 2);
        assert_eq!(tree.child_count(WidgetId::from_u64(4)), 0);
    }
}
