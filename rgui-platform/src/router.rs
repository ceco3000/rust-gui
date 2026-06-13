//! 事件路由器——事件分发（D5 §3）。
//!
//! 占位模块——完整实现需 winit 事件循环集成。

use crate::event::Event;
use rgui_core::id::WidgetId;
use rustc_hash::FxHashMap;

/// 事件路由器（D5 §3.1）。
///
/// 管理 widget 树结构，将事件路由到目标 widget。
pub struct EventRouter {
    /// widget → 父 widget。
    parent: FxHashMap<WidgetId, WidgetId>,
    /// widget → 子 widget 列表。
    children: FxHashMap<WidgetId, Vec<WidgetId>>,
}

impl EventRouter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            parent: FxHashMap::default(),
            children: FxHashMap::default(),
        }
    }

    /// 添加父子关系。
    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        self.parent.insert(child, parent);
        self.children.entry(parent).or_default().push(child);
    }

    /// 移除 widget 及其后代。
    pub fn remove(&mut self, widget_id: WidgetId) {
        if let Some(children) = self.children.remove(&widget_id) {
            for child in &children {
                self.parent.remove(child);
                self.remove(*child);
            }
        }
        self.parent.remove(&widget_id);
    }

    /// 获取从根到目标的祖先链（用于捕获/冒泡）。
    #[must_use]
    pub fn ancestors(&self, widget_id: WidgetId) -> Vec<WidgetId> {
        let mut path = Vec::new();
        let mut current = Some(widget_id);
        while let Some(id) = current {
            path.push(id);
            current = self.parent.get(&id).copied();
        }
        path.reverse(); // 根在前
        path
    }

    /// 判断是否是有效的聚焦目标（widget 在树中）。
    #[must_use]
    pub fn is_valid_target(&self, _event: &Event, _widget_id: WidgetId) -> bool {
        self.parent.contains_key(&_widget_id) || self.children.contains_key(&_widget_id)
    }
}

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EventRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventRouter")
            .field("widgets", &self.parent.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_add_child() {
        let mut router = EventRouter::new();
        let parent = WidgetId::from_u64(1);
        let child = WidgetId::from_u64(2);
        router.add_child(parent, child);
        assert!(router.parent.contains_key(&child));
    }

    #[test]
    fn router_ancestors() {
        let mut router = EventRouter::new();
        let root = WidgetId::from_u64(1);
        let child = WidgetId::from_u64(2);
        let grandchild = WidgetId::from_u64(3);

        router.add_child(root, child);
        router.add_child(child, grandchild);

        let ancestors = router.ancestors(grandchild);
        assert_eq!(ancestors, vec![root, child, grandchild]);
    }

    #[test]
    fn router_remove_cascades() {
        let mut router = EventRouter::new();
        router.add_child(WidgetId::from_u64(1), WidgetId::from_u64(2));
        router.add_child(WidgetId::from_u64(2), WidgetId::from_u64(3));
        router.remove(WidgetId::from_u64(2));
        // child 和 grandchild 都应被移除
        assert!(!router.parent.contains_key(&WidgetId::from_u64(2)));
        assert!(!router.parent.contains_key(&WidgetId::from_u64(3)));
    }
}
