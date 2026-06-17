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

use rgui_core::geometry::Rect;
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
    /// widget → 布局边界矩形（D5 §3.1）。
    /// 由布局系统填充，供 hit_test 使用。
    bounds: FxHashMap<WidgetId, Rect>,
}

impl WidgetTree {
    /// 创建空的 widget 树。
    #[must_use]
    pub fn new() -> Self {
        Self {
            parent: FxHashMap::default(),
            children: FxHashMap::default(),
            bounds: FxHashMap::default(),
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
                self.bounds.remove(child);
                self.remove(*child);
            }
        }
        // 从父节点的 children 列表中移除自己
        if let Some(parent_id) = self.parent.remove(&widget_id) {
            if let Some(siblings) = self.children.get_mut(&parent_id) {
                siblings.retain(|&id| id != widget_id);
            }
        }
        // 移除边界信息
        self.bounds.remove(&widget_id);
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
        self.parent.contains_key(&widget_id)
            || self.children.contains_key(&widget_id)
            || self.bounds.contains_key(&widget_id)
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

    // ── 布局边界（D5 §3.1/§4）──────────────────────────────────────────

    /// 设置 widget 的布局边界矩形。
    pub fn set_bounds(&mut self, widget_id: WidgetId, bounds: Rect) {
        self.bounds.insert(widget_id, bounds);
    }

    /// 获取 widget 的布局边界矩形。
    #[must_use]
    pub fn get_bounds(&self, widget_id: WidgetId) -> Option<Rect> {
        self.bounds.get(&widget_id).copied()
    }

    /// 移除 widget 的布局边界信息。
    pub fn remove_bounds(&mut self, widget_id: WidgetId) {
        self.bounds.remove(&widget_id);
    }

    // ── 树结构变更 ─────────────────────────────────────────────────────

    /// 将 widget 从其当前父节点移动到新父节点的指定索引位置。
    ///
    /// 此方法处理 `Patch::MoveWidget` 对应的树结构更新：
    /// 1. 从旧父节点的 children 列表中移除
    /// 2. 更新 parent 映射
    /// 3. 插入到新父节点的 children 列表的指定位置
    ///
    /// 如果 widget 当前没有父节点（根节点），则只设置新的 parent 关系。
    pub fn reparent(&mut self, widget_id: WidgetId, new_parent: WidgetId, new_index: usize) {
        // 从旧父节点的 children 列表中移除
        if let Some(old_parent) = self.parent.get(&widget_id).copied() {
            if let Some(siblings) = self.children.get_mut(&old_parent) {
                siblings.retain(|&id| id != widget_id);
            }
        }

        // 更新 parent 映射
        self.parent.insert(widget_id, new_parent);

        // 插入到新父节点的 children 列表
        let siblings = self.children.entry(new_parent).or_default();
        let index = new_index.min(siblings.len());
        siblings.insert(index, widget_id);
    }

    /// 根据 patch 结果同步 WidgetTree 的 parent/children 关系。
    ///
    /// 此方法将 `apply_patch` 返回的 `ApplyResult` 中的结构变更
    /// 应用到 WidgetTree，使 parent/children 映射与实际 widget 树一致。
    ///
    /// - `created`: (widget_id, parent_id, index) — 调用 `add_child`
    /// - `removed`: widget_id 列表 — 调用 `remove`（含级联删除）
    /// - `moved`: (widget_id, new_parent, new_index) — 调用 `reparent`
    ///
    /// 注意：bounds 不由 patch 填充，而由布局系统在布局阶段通过
    /// `set_bounds` 单独设置。此方法仅同步父子关系。
    pub fn sync_from_patch(
        &mut self,
        created: &[(WidgetId, WidgetId, usize)],
        removed: &[WidgetId],
        moved: &[(WidgetId, WidgetId, usize)],
    ) {
        // 先处理移除（可能产生级联效果）
        for &widget_id in removed {
            self.remove(widget_id);
        }

        // 处理创建
        for &(widget_id, parent_id, _index) in created {
            self.add_child(parent_id, widget_id);
        }

        // 处理移动（需在创建之后，因为目标父节点可能刚被创建）
        for &(widget_id, new_parent, new_index) in moved {
            self.reparent(widget_id, new_parent, new_index);
        }
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

    // ── bounds ────────────────────────────────────────────────────────

    #[test]
    fn set_and_get_bounds() {
        let mut tree = WidgetTree::new();
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        tree.set_bounds(WidgetId::from_u64(1), rect);
        assert_eq!(tree.get_bounds(WidgetId::from_u64(1)), Some(rect));
    }

    #[test]
    fn get_bounds_unknown_returns_none() {
        let tree = WidgetTree::new();
        assert_eq!(tree.get_bounds(WidgetId::from_u64(99)), None);
    }

    #[test]
    fn remove_bounds_clears_entry() {
        let mut tree = WidgetTree::new();
        tree.set_bounds(WidgetId::from_u64(1), Rect::new(0.0, 0.0, 100.0, 50.0));
        tree.remove_bounds(WidgetId::from_u64(1));
        assert_eq!(tree.get_bounds(WidgetId::from_u64(1)), None);
    }

    #[test]
    fn contains_detects_widget_with_only_bounds() {
        let mut tree = WidgetTree::new();
        // widget 只有 bounds，没有 parent/children 关系
        tree.set_bounds(WidgetId::from_u64(42), Rect::new(10.0, 10.0, 50.0, 30.0));
        assert!(tree.contains(WidgetId::from_u64(42)));
    }

    #[test]
    fn remove_cleans_up_bounds() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        tree.set_bounds(WidgetId::from_u64(2), Rect::new(0.0, 0.0, 100.0, 50.0));
        tree.remove(WidgetId::from_u64(2));
        assert!(!tree.contains(WidgetId::from_u64(2)));
        assert_eq!(tree.get_bounds(WidgetId::from_u64(2)), None);
    }

    #[test]
    fn remove_cascade_cleans_bounds_of_descendants() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        tree.add_child(WidgetId::from_u64(2), WidgetId::from_u64(3));
        tree.set_bounds(WidgetId::from_u64(2), Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.set_bounds(WidgetId::from_u64(3), Rect::new(10.0, 10.0, 180.0, 80.0));

        tree.remove(WidgetId::from_u64(2));
        // 子节点也消失
        assert!(!tree.contains(WidgetId::from_u64(3)));
        assert_eq!(tree.get_bounds(WidgetId::from_u64(2)), None);
        assert_eq!(tree.get_bounds(WidgetId::from_u64(3)), None);
    }

    // ── reparent ──────────────────────────────────────────────────────

    #[test]
    fn reparent_moves_widget_to_new_parent() {
        let mut tree = WidgetTree::new();
        // 初始：root(1) → child(2), root(1) → other(3)
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(3));

        // 将 2 移动到 3 的子节点
        tree.reparent(WidgetId::from_u64(2), WidgetId::from_u64(3), 0);

        assert_eq!(
            tree.parent(WidgetId::from_u64(2)),
            Some(WidgetId::from_u64(3))
        );
        assert_eq!(
            tree.children(WidgetId::from_u64(1)),
            &[WidgetId::from_u64(3)]
        );
        assert_eq!(
            tree.children(WidgetId::from_u64(3)),
            &[WidgetId::from_u64(2)]
        );
    }

    #[test]
    fn reparent_inserts_at_specified_index() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(10));
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(30));

        // 在索引 1 处（即 10 和 30 之间）插入
        tree.reparent(WidgetId::from_u64(20), WidgetId::from_u64(1), 1);

        assert_eq!(
            tree.children(WidgetId::from_u64(1)),
            &[
                WidgetId::from_u64(10),
                WidgetId::from_u64(20),
                WidgetId::from_u64(30),
            ]
        );
    }

    #[test]
    fn reparent_orphan_sets_parent() {
        // widget 当前没有父节点时也能正常工作
        let mut tree = WidgetTree::new();
        tree.reparent(WidgetId::from_u64(42), WidgetId::from_u64(1), 0);
        assert_eq!(
            tree.parent(WidgetId::from_u64(42)),
            Some(WidgetId::from_u64(1))
        );
        assert_eq!(
            tree.children(WidgetId::from_u64(1)),
            &[WidgetId::from_u64(42)]
        );
    }

    // ── sync_from_patch ───────────────────────────────────────────────

    #[test]
    fn sync_from_patch_create_widgets() {
        let mut tree = WidgetTree::new();
        let root = WidgetId::from_u64(1);
        let child_a = WidgetId::from_u64(2);
        let child_b = WidgetId::from_u64(3);

        tree.sync_from_patch(&[(child_a, root, 0), (child_b, root, 1)], &[], &[]);

        assert_eq!(tree.parent(child_a), Some(root));
        assert_eq!(tree.parent(child_b), Some(root));
        // root 作为父节点自然出现在 children 中
        assert_eq!(tree.children(root), &[child_a, child_b]);
    }

    #[test]
    fn sync_from_patch_remove_widgets() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        tree.add_child(WidgetId::from_u64(2), WidgetId::from_u64(3));
        assert!(tree.contains(WidgetId::from_u64(2)));

        tree.sync_from_patch(&[], &[WidgetId::from_u64(2)], &[]);

        assert!(!tree.contains(WidgetId::from_u64(2)));
        assert!(!tree.contains(WidgetId::from_u64(3)));
    }

    #[test]
    fn sync_from_patch_move_widgets() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(3));

        tree.sync_from_patch(
            &[],
            &[],
            &[(WidgetId::from_u64(2), WidgetId::from_u64(3), 0)],
        );

        assert_eq!(
            tree.parent(WidgetId::from_u64(2)),
            Some(WidgetId::from_u64(3))
        );
        assert_eq!(
            tree.children(WidgetId::from_u64(3)),
            &[WidgetId::from_u64(2)]
        );
    }

    #[test]
    fn sync_from_patch_mixed_operations() {
        let mut tree = WidgetTree::new();
        let root = WidgetId::from_u64(1);

        // 创建 a, b
        let a = WidgetId::from_u64(10);
        let b = WidgetId::from_u64(20);
        let c = WidgetId::from_u64(30);

        tree.sync_from_patch(&[(a, root, 0), (b, root, 1)], &[], &[]);
        assert_eq!(tree.children(root), &[a, b]);

        // 移除 b，创建 c，移动 a 到 c 的子节点
        tree.sync_from_patch(&[(c, root, 0)], &[b], &[(a, c, 0)]);

        assert!(!tree.contains(b));
        assert_eq!(tree.children(root), &[c]);
        assert_eq!(tree.parent(a), Some(c));
        assert_eq!(tree.children(c), &[a]);
    }

    #[test]
    fn sync_from_patch_empty_lists_noop() {
        let mut tree = WidgetTree::new();
        tree.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        let len_before = tree.len();

        tree.sync_from_patch(&[], &[], &[]);

        assert_eq!(tree.len(), len_before);
        assert!(tree.contains(WidgetId::from_u64(2)));
    }
}
