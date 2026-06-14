//! WidgetView diff 算法与 Patch 机制。
//!
//! 定义源自 D2 §5。

use rgui_core::id::WidgetId;
use rgui_core::traits::AppMessage;
use rgui_core::view::{Key, MessageBinding, PropValue, WidgetView};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

// ============================================================================
// Patch
// ============================================================================

/// Widget 树补丁：描述从旧视图到新视图的变化（D2 §5.2）。
#[derive(Clone, Debug)]
pub enum Patch<M: AppMessage> {
    /// 创建新 widget 节点。
    CreateWidget {
        parent: WidgetId,
        index: usize,
        widget_type: &'static str,
        widget_id: WidgetId,
        props: BTreeMap<&'static str, PropValue>,
        message_bindings: Vec<MessageBinding<M>>,
    },
    /// 更新已有 widget 的属性。
    UpdateProps {
        widget_id: WidgetId,
        changed: BTreeMap<&'static str, PropValue>,
        removed: Vec<&'static str>,
    },
    /// 更新消息绑定。
    UpdateBindings {
        widget_id: WidgetId,
        bindings: Vec<MessageBinding<M>>,
    },
    /// 移动 widget 到新位置。
    MoveWidget {
        widget_id: WidgetId,
        new_parent: WidgetId,
        new_index: usize,
    },
    /// 移除 widget（及其子节点）。
    RemoveWidget { widget_id: WidgetId },
    /// 替换 widget 类型。
    ReplaceWidget {
        widget_id: WidgetId,
        new_widget_type: &'static str,
        props: BTreeMap<&'static str, PropValue>,
        message_bindings: Vec<MessageBinding<M>>,
    },
    /// 批量 patch。
    Batch(Vec<Patch<M>>),
}

// ============================================================================
// WidgetIdMap
// ============================================================================

/// Widget ID 查找表（D2 §5.5）。
///
/// 在 diff 过程中维护路径 → WidgetId 映射。
#[derive(Debug, Default)]
pub struct WidgetIdMap {
    path_to_id: FxHashMap<WidgetPath, WidgetId>,
    stable_id_to_id: FxHashMap<&'static str, WidgetId>,
    next_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WidgetPath {
    parent: WidgetId,
    index: usize,
}

impl WidgetIdMap {
    /// 创建空的 WidgetIdMap。
    #[must_use]
    pub fn new() -> Self {
        Self {
            path_to_id: FxHashMap::default(),
            stable_id_to_id: FxHashMap::default(),
            next_id: 0,
        }
    }

    /// 按路径查找 WidgetId。
    #[must_use]
    pub fn get_by_path(&self, parent: WidgetId, index: usize) -> Option<WidgetId> {
        self.path_to_id.get(&WidgetPath { parent, index }).copied()
    }

    /// 分配新的 WidgetId。
    pub fn allocate(&mut self) -> WidgetId {
        self.next_id += 1;
        WidgetId::from_u64(self.next_id)
    }

    /// 记录路径 → WidgetId 映射。
    pub fn insert_path(&mut self, parent: WidgetId, index: usize, id: WidgetId) {
        self.path_to_id.insert(WidgetPath { parent, index }, id);
    }

    /// 记录 stable_id → WidgetId 映射。
    pub fn insert_stable_id(&mut self, stable_id: &'static str, id: WidgetId) {
        self.stable_id_to_id.insert(stable_id, id);
    }
}

// ============================================================================
// resolve_id
// ============================================================================

/// 解析节点身份（D2 §5.3）。
fn resolve_id<M: AppMessage>(
    old: &WidgetView<M>,
    new: &WidgetView<M>,
    parent_id: WidgetId,
    child_index: usize,
    id_map: &mut WidgetIdMap,
) -> WidgetId {
    // 1. 通过 stable ID 查找（new.id 或 old.id）
    if let Some(stable_id) = new.id.or(old.id) {
        let name = stable_id.as_u64().to_string();
        let leaked: &'static str = Box::leak(name.into_boxed_str());
        if let Some(existing) = id_map.stable_id_to_id.get(leaked) {
            return *existing;
        }
    }

    // 2. 按路径查找
    if let Some(existing) = id_map.get_by_path(parent_id, child_index) {
        return existing;
    }

    // 3. 分配新 ID
    id_map.allocate()
}

// ============================================================================
// diff
// ============================================================================

/// 比较新旧 WidgetView 树，产生 Patch 列表（D2 §5.3）。
pub fn diff<M: AppMessage>(
    old_view: &WidgetView<M>,
    new_view: &WidgetView<M>,
    parent_id: WidgetId,
    id_map: &mut WidgetIdMap,
) -> Vec<Patch<M>> {
    let mut patches = Vec::new();
    diff_recursive(old_view, new_view, parent_id, 0, id_map, &mut patches);
    patches
}

fn diff_recursive<M: AppMessage>(
    old: &WidgetView<M>,
    new: &WidgetView<M>,
    parent_id: WidgetId,
    child_index: usize,
    id_map: &mut WidgetIdMap,
    patches: &mut Vec<Patch<M>>,
) {
    let widget_id = resolve_id(old, new, parent_id, child_index, id_map);
    id_map.insert_path(parent_id, child_index, widget_id);

    // 类型变更 → 完全替换
    if old.widget_type != new.widget_type {
        patches.push(Patch::ReplaceWidget {
            widget_id,
            new_widget_type: new.widget_type,
            props: new.props.clone(),
            message_bindings: new.message_bindings.clone(),
        });
        return;
    }

    // 属性 diff
    if let Some(prop_patch) = diff_props(&old.props, &new.props) {
        patches.push(Patch::UpdateProps {
            widget_id,
            changed: prop_patch.changed,
            removed: prop_patch.removed,
        });
    }

    // 消息绑定 diff
    if old.message_bindings != new.message_bindings {
        patches.push(Patch::UpdateBindings {
            widget_id,
            bindings: new.message_bindings.clone(),
        });
    }

    // 子节点 reconciliation
    let child_patches = reconcile_children(&old.children, &new.children, widget_id, id_map);
    patches.extend(child_patches);
}

// ============================================================================
// diff_props
// ============================================================================

/// 属性 diff 结果。
#[derive(Debug, PartialEq)]
pub struct PropDiff {
    pub changed: BTreeMap<&'static str, PropValue>,
    pub removed: Vec<&'static str>,
}

/// 比较两个属性映射（D2 §5.4）。
#[must_use]
pub fn diff_props(
    old: &BTreeMap<&'static str, PropValue>,
    new: &BTreeMap<&'static str, PropValue>,
) -> Option<PropDiff> {
    let mut changed = BTreeMap::new();
    let mut removed = Vec::new();

    for (key, new_val) in new {
        match old.get(key) {
            Some(old_val) if old_val == new_val => {},
            _ => {
                changed.insert(*key, new_val.clone());
            },
        }
    }

    for key in old.keys() {
        if !new.contains_key(key) {
            removed.push(*key);
        }
    }

    if changed.is_empty() && removed.is_empty() {
        None
    } else {
        Some(PropDiff { changed, removed })
    }
}

// ============================================================================
// reconcile_children
// ============================================================================

/// 子节点 reconciliation（D2 §5.4）。
fn reconcile_children<M: AppMessage>(
    old_children: &[WidgetView<M>],
    new_children: &[WidgetView<M>],
    parent_id: WidgetId,
    id_map: &mut WidgetIdMap,
) -> Vec<Patch<M>> {
    let all_have_keys = new_children.iter().all(|c| c.key.is_some())
        && old_children.iter().all(|c| c.key.is_some());

    if all_have_keys {
        keyed_reconciliation(old_children, new_children, parent_id, id_map)
    } else {
        positional_reconciliation(old_children, new_children, parent_id, id_map)
    }
}

/// 基于 key 的 reconciliation。
fn keyed_reconciliation<M: AppMessage>(
    old_children: &[WidgetView<M>],
    new_children: &[WidgetView<M>],
    parent_id: WidgetId,
    id_map: &mut WidgetIdMap,
) -> Vec<Patch<M>> {
    let mut patches = Vec::new();

    // 构建旧 key 索引
    let mut old_by_key: FxHashMap<Key, (usize, WidgetId)> = FxHashMap::default();
    for (i, c) in old_children.iter().enumerate() {
        if let Some(ref key) = c.key {
            if let Some(id) = id_map.get_by_path(parent_id, i) {
                old_by_key.insert(key.clone(), (i, id));
            }
        }
    }

    // 构建新 key 集合
    let mut new_keys: Vec<&Key> = Vec::new();
    for c in new_children {
        if let Some(ref key) = c.key {
            new_keys.push(key);
        }
    }

    // 删除：旧 key 不在新列表中
    for (key, (_idx, id)) in &old_by_key {
        if !new_keys.contains(&key) {
            patches.push(Patch::RemoveWidget { widget_id: *id });
        }
    }

    // 新增 + 更新
    for (new_idx, new_child) in new_children.iter().enumerate() {
        if let Some(ref key) = new_child.key {
            if let Some(&(old_idx, _old_id)) = old_by_key.get(key) {
                diff_recursive(
                    &old_children[old_idx],
                    new_child,
                    parent_id,
                    new_idx,
                    id_map,
                    &mut patches,
                );
            } else {
                let new_id = id_map.allocate();
                patches.push(Patch::CreateWidget {
                    parent: parent_id,
                    index: new_idx,
                    widget_type: new_child.widget_type,
                    widget_id: new_id,
                    props: new_child.props.clone(),
                    message_bindings: new_child.message_bindings.clone(),
                });
            }
        }
    }

    patches
}

/// 基于位置的 reconciliation。
fn positional_reconciliation<M: AppMessage>(
    old_children: &[WidgetView<M>],
    new_children: &[WidgetView<M>],
    parent_id: WidgetId,
    id_map: &mut WidgetIdMap,
) -> Vec<Patch<M>> {
    let mut patches = Vec::new();
    let max_len = old_children.len().max(new_children.len());

    for i in 0..max_len {
        match (old_children.get(i), new_children.get(i)) {
            (Some(old), Some(new)) => {
                diff_recursive(old, new, parent_id, i, id_map, &mut patches);
            },
            (None, Some(new)) => {
                let new_id = id_map.allocate();
                patches.push(Patch::CreateWidget {
                    parent: parent_id,
                    index: i,
                    widget_type: new.widget_type,
                    widget_id: new_id,
                    props: new.props.clone(),
                    message_bindings: new.message_bindings.clone(),
                });
            },
            (Some(_old), None) => {
                if let Some(id) = id_map.get_by_path(parent_id, i) {
                    patches.push(Patch::RemoveWidget { widget_id: id });
                }
            },
            (None, None) => unreachable!(),
        }
    }

    patches
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::id::WidgetId;

    /// 测试用 AppMessage
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[allow(dead_code)]
    enum TestMsg {
        Nop,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            match self {
                Self::Nop => "nop",
            }
        }
    }

    fn make_view(widget_type: &'static str) -> WidgetView<TestMsg> {
        WidgetView::new(widget_type)
    }

    fn make_view_with_props(
        widget_type: &'static str,
        props: BTreeMap<&'static str, PropValue>,
    ) -> WidgetView<TestMsg> {
        WidgetView::new(widget_type).props(props)
    }

    // --- diff_props ---

    #[test]
    fn diff_props_no_change() {
        let mut props = BTreeMap::new();
        props.insert("text", PropValue::str("hello"));
        assert_eq!(diff_props(&props, &props), None);
    }

    #[test]
    fn diff_props_changed() {
        let mut old = BTreeMap::new();
        old.insert("text", PropValue::str("hello"));

        let mut new = BTreeMap::new();
        new.insert("text", PropValue::str("world"));

        let result = diff_props(&old, &new).unwrap();
        assert_eq!(result.changed.len(), 1);
        assert!(result.removed.is_empty());
    }

    #[test]
    fn diff_props_removed() {
        let mut old = BTreeMap::new();
        old.insert("text", PropValue::str("hello"));
        old.insert("count", PropValue::Int(5));

        let mut new = BTreeMap::new();
        new.insert("text", PropValue::str("hello"));

        let result = diff_props(&old, &new).unwrap();
        assert_eq!(result.removed, vec!["count"]);
        assert!(result.changed.is_empty());
    }

    #[test]
    fn diff_props_added() {
        let mut old = BTreeMap::new();
        old.insert("text", PropValue::str("hello"));

        let mut new = BTreeMap::new();
        new.insert("text", PropValue::str("hello"));
        new.insert("enabled", PropValue::Bool(true));

        let result = diff_props(&old, &new).unwrap();
        assert_eq!(result.changed.len(), 1);
        assert!(result.changed.contains_key("enabled"));
    }

    // --- WidgetView diff ---

    #[test]
    fn diff_no_change() {
        let view = make_view("label");
        let mut id_map = WidgetIdMap::new();
        let parent = WidgetId::from_u64(1);
        let patches = diff(&view, &view, parent, &mut id_map);
        assert!(patches.is_empty());
    }

    #[test]
    fn diff_type_changed() {
        let old = make_view("label");
        let new = make_view("button");
        let mut id_map = WidgetIdMap::new();
        let parent = WidgetId::from_u64(1);
        let patches = diff(&old, &new, parent, &mut id_map);
        assert_eq!(patches.len(), 1);
        match &patches[0] {
            Patch::ReplaceWidget {
                new_widget_type, ..
            } => {
                assert_eq!(*new_widget_type, "button");
            },
            other => panic!("期望 ReplaceWidget，得到 {other:?}"),
        }
    }

    #[test]
    fn diff_props_changed_in_view() {
        let old = make_view_with_props("button", {
            let mut m = BTreeMap::new();
            m.insert("text", PropValue::str("旧文本"));
            m
        });
        let new = make_view_with_props("button", {
            let mut m = BTreeMap::new();
            m.insert("text", PropValue::str("新文本"));
            m
        });
        let mut id_map = WidgetIdMap::new();
        let parent = WidgetId::from_u64(1);
        let patches = diff(&old, &new, parent, &mut id_map);
        assert_eq!(patches.len(), 1);
        match &patches[0] {
            Patch::UpdateProps { changed, .. } => {
                assert!(changed.contains_key("text"));
            },
            other => panic!("期望 UpdateProps，得到 {other:?}"),
        }
    }

    #[test]
    fn diff_child_added() {
        let old = make_view("column");

        let child = make_view("label");
        let new = make_view("column").child(child);

        let mut id_map = WidgetIdMap::new();
        let parent = WidgetId::from_u64(1);
        let patches = diff(&old, &new, parent, &mut id_map);

        assert!(!patches.is_empty());
        match &patches[0] {
            Patch::CreateWidget { widget_type, .. } => {
                assert_eq!(*widget_type, "label");
            },
            other => panic!("期望 CreateWidget，得到 {other:?}"),
        }
    }

    #[test]
    fn diff_child_removed() {
        let child = make_view("label");
        let old = make_view("column").child(child);
        let new = make_view("column");

        let mut id_map = WidgetIdMap::new();
        let parent = WidgetId::from_u64(1);
        let patches = diff(&old, &new, parent, &mut id_map);
        assert!(!patches.is_empty());
    }

    // --- WidgetIdMap ---

    #[test]
    fn id_map_allocate_unique() {
        let mut map = WidgetIdMap::new();
        let id1 = map.allocate();
        let id2 = map.allocate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn id_map_get_by_path() {
        let mut map = WidgetIdMap::new();
        let parent = WidgetId::from_u64(1);
        let id = WidgetId::from_u64(100);
        map.insert_path(parent, 0, id);
        assert_eq!(map.get_by_path(parent, 0), Some(id));
        assert_eq!(map.get_by_path(parent, 1), None);
    }
}
