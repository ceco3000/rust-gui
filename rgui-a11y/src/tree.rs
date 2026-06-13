//! 无障碍树——AccessibilityTree（D6）。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::id::WidgetId;
use rustc_hash::FxHashMap;

/// 无障碍树更新描述。
#[derive(Debug, Clone)]
pub struct TreeUpdate {
    pub node: AccessibilityNode,
    pub children: Vec<WidgetId>,
}

/// 无障碍树。
///
/// 存储整个 widget 树的无障碍节点，支持增量更新。
pub struct AccessibilityTree {
    nodes: FxHashMap<WidgetId, AccessibilityNode>,
    root: Option<WidgetId>,
}

impl AccessibilityTree {
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: FxHashMap::default(), root: None }
    }

    /// 更新或插入无障碍节点。
    pub fn upsert(&mut self, widget_id: WidgetId, node: AccessibilityNode) {
        self.nodes.insert(widget_id, node);
    }

    /// 移除节点。
    pub fn remove(&mut self, widget_id: WidgetId) {
        self.nodes.remove(&widget_id);
    }

    /// 设置根节点。
    pub fn set_root(&mut self, widget_id: WidgetId) {
        self.root = Some(widget_id);
    }

    /// 获取节点。
    #[must_use]
    pub fn get(&self, widget_id: WidgetId) -> Option<&AccessibilityNode> {
        self.nodes.get(&widget_id)
    }

    /// 节点数量。
    #[must_use]
    pub fn len(&self) -> usize { self.nodes.len() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

    /// 清除所有节点。
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root = None;
    }
}

impl Default for AccessibilityTree {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for AccessibilityTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessibilityTree")
            .field("nodes", &self.nodes.len())
            .field("root", &self.root)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_upsert_and_get() {
        let mut tree = AccessibilityTree::new();
        let id = WidgetId::from_u64(1);
        let node = AccessibilityNode::none();
        tree.upsert(id, node.clone());
        assert!(tree.get(id).is_some());
    }

    #[test]
    fn tree_remove() {
        let mut tree = AccessibilityTree::new();
        let id = WidgetId::from_u64(1);
        tree.upsert(id, AccessibilityNode::none());
        tree.remove(id);
        assert!(tree.get(id).is_none());
    }

    #[test]
    fn tree_is_empty() {
        let tree = AccessibilityTree::new();
        assert!(tree.is_empty());
    }
}
