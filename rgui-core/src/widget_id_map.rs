//! `WidgetIdBimap` — WidgetId 与字符串 id 的双向映射。
//!
//! ## 用途
//!
//! `.rgui` 解析器中 `id="xxx"` 属性建立字符串→WidgetId 映射，
//! 同时维护反向映射供 `handle_click` 将 widget_id 转回字符串 id。
//!
//! - **正向** (`String → WidgetId`): Rhai `set_prop("section1", ...)` 查找 widget
//! - **反向** (`WidgetId → String`): 点击事件路由时反查字符串 id
//!
//! ## 设计约束
//!
//! - 一个字符串 id 只能映射到一个 WidgetId（id 在 `.rgui` 文档内唯一）
//! - 同一个 WidgetId 重复 insert 时，旧的正向映射被删除，反向映射更新
//!
//! # 示例
//!
//! ```
//! use rgui_core::widget_id_map::WidgetIdBimap;
//! use rgui_core::id::WidgetId;
//!
//! let mut bimap = WidgetIdBimap::new();
//! let id = WidgetId::from_u64(42);
//! bimap.insert("section1", id);
//!
//! assert_eq!(bimap.get_id("section1"), Some(id));
//! assert_eq!(bimap.get_name(id), Some("section1"));
//! ```

use std::collections::HashMap;

use crate::id::WidgetId;

/// WidgetId 与字符串 id 的双向映射表。
///
/// 维护正向 (`String → WidgetId`) 和反向 (`WidgetId → String`) 两套索引，
/// 支持 O(1) 双向查询。
#[derive(Clone, Debug, Default)]
pub struct WidgetIdBimap {
    /// 正向映射：字符串 id → WidgetId
    forward: HashMap<String, WidgetId>,
    /// 反向映射：WidgetId → 字符串 id
    reverse: HashMap<WidgetId, String>,
}

impl WidgetIdBimap {
    /// 创建一个空的双向映射表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// 插入一个字符串 id → WidgetId 映射。
    ///
    /// 如果该字符串 id 已映射到另一个 WidgetId，旧映射被替换。
    /// 如果该 WidgetId 已有另一个字符串 id，旧的反向映射也被替换。
    pub fn insert(&mut self, name: &str, id: WidgetId) {
        // 如果此字符串 id 之前映射到其他 WidgetId，清除旧反向映射
        if let Some(&old_id) = self.forward.get(name) {
            if old_id != id {
                self.reverse.remove(&old_id);
            }
        }
        // 如果此 WidgetId 之前有另一个字符串 id，清除旧正向映射
        if let Some(old_name) = self.reverse.get(&id) {
            if old_name != name {
                self.forward.remove(old_name);
            }
        }
        self.forward.insert(name.to_string(), id);
        self.reverse.insert(id, name.to_string());
    }

    /// 通过字符串 id 查找 WidgetId。
    #[must_use]
    pub fn get_id(&self, name: &str) -> Option<WidgetId> {
        self.forward.get(name).copied()
    }

    /// 通过 WidgetId 查找字符串 id。
    #[must_use]
    pub fn get_name(&self, id: WidgetId) -> Option<&str> {
        self.reverse.get(&id).map(String::as_str)
    }

    /// 移除一个字符串 id 及其对应的 WidgetId 映射。
    ///
    /// 返回被移除的 WidgetId（如果存在）。
    pub fn remove_by_name(&mut self, name: &str) -> Option<WidgetId> {
        let id = self.forward.remove(name)?;
        self.reverse.remove(&id);
        Some(id)
    }

    /// 移除一个 WidgetId 及其对应的字符串 id 映射。
    ///
    /// 返回被移除的字符串 id（如果存在）。
    pub fn remove_by_id(&mut self, id: WidgetId) -> Option<String> {
        let name = self.reverse.remove(&id)?;
        self.forward.remove(&name);
        Some(name)
    }

    /// 双向映射中的条目数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.forward.len()
    }

    /// 是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }

    /// 清除所有映射。
    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
    }

    /// 检查字符串 id 是否已存在。
    #[must_use]
    pub fn contains_name(&self, name: &str) -> bool {
        self.forward.contains_key(name)
    }

    /// 检查 WidgetId 是否已存在。
    #[must_use]
    pub fn contains_id(&self, id: WidgetId) -> bool {
        self.reverse.contains_key(&id)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_bimap() {
        let bimap = WidgetIdBimap::new();
        assert!(bimap.is_empty());
        assert_eq!(bimap.len(), 0);
    }

    #[test]
    fn insert_and_get_id_roundtrip() {
        let mut bimap = WidgetIdBimap::new();
        let id = WidgetId::from_u64(42);
        bimap.insert("section1", id);
        assert_eq!(bimap.get_id("section1"), Some(id));
    }

    #[test]
    fn insert_and_get_name_roundtrip() {
        let mut bimap = WidgetIdBimap::new();
        let id = WidgetId::from_u64(42);
        bimap.insert("section1", id);
        assert_eq!(bimap.get_name(id), Some("section1"));
    }

    #[test]
    fn get_id_nonexistent_returns_none() {
        let bimap = WidgetIdBimap::new();
        assert_eq!(bimap.get_id("nonexistent"), None);
    }

    #[test]
    fn get_name_nonexistent_returns_none() {
        let bimap = WidgetIdBimap::new();
        assert_eq!(bimap.get_name(WidgetId::from_u64(999)), None);
    }

    #[test]
    fn insert_multiple_entries() {
        let mut bimap = WidgetIdBimap::new();
        let id1 = WidgetId::from_u64(1);
        let id2 = WidgetId::from_u64(2);
        let id3 = WidgetId::from_u64(3);

        bimap.insert("a", id1);
        bimap.insert("b", id2);
        bimap.insert("c", id3);

        assert_eq!(bimap.len(), 3);
        assert_eq!(bimap.get_id("a"), Some(id1));
        assert_eq!(bimap.get_id("b"), Some(id2));
        assert_eq!(bimap.get_id("c"), Some(id3));
        assert_eq!(bimap.get_name(id1), Some("a"));
        assert_eq!(bimap.get_name(id2), Some("b"));
        assert_eq!(bimap.get_name(id3), Some("c"));
    }

    #[test]
    fn insert_same_name_overwrites_old_id() {
        let mut bimap = WidgetIdBimap::new();
        let old_id = WidgetId::from_u64(10);
        let new_id = WidgetId::from_u64(20);

        bimap.insert("widget", old_id);
        bimap.insert("widget", new_id);

        // 正向映射指向新 id
        assert_eq!(bimap.get_id("widget"), Some(new_id));
        // 旧 id 的反向映射被清除
        assert_eq!(bimap.get_name(old_id), None);
        // 新 id 的反向映射存在
        assert_eq!(bimap.get_name(new_id), Some("widget"));
        assert_eq!(bimap.len(), 1);
    }

    #[test]
    fn insert_same_id_overwrites_old_name() {
        let mut bimap = WidgetIdBimap::new();
        let id = WidgetId::from_u64(42);

        bimap.insert("old_name", id);
        bimap.insert("new_name", id);

        // 旧字符串名不再有效
        assert_eq!(bimap.get_id("old_name"), None);
        // 新字符串名有效
        assert_eq!(bimap.get_id("new_name"), Some(id));
        assert_eq!(bimap.get_name(id), Some("new_name"));
        assert_eq!(bimap.len(), 1);
    }

    #[test]
    fn remove_by_name_clears_both_directions() {
        let mut bimap = WidgetIdBimap::new();
        let id = WidgetId::from_u64(42);
        bimap.insert("section1", id);

        let removed = bimap.remove_by_name("section1");
        assert_eq!(removed, Some(id));
        assert!(bimap.is_empty());
        assert_eq!(bimap.get_id("section1"), None);
        assert_eq!(bimap.get_name(id), None);
    }

    #[test]
    fn remove_by_id_clears_both_directions() {
        let mut bimap = WidgetIdBimap::new();
        let id = WidgetId::from_u64(42);
        bimap.insert("section1", id);

        let removed = bimap.remove_by_id(id);
        assert_eq!(removed, Some("section1".to_string()));
        assert!(bimap.is_empty());
        assert_eq!(bimap.get_id("section1"), None);
        assert_eq!(bimap.get_name(id), None);
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut bimap = WidgetIdBimap::new();
        assert_eq!(bimap.remove_by_name("ghost"), None);
        assert_eq!(bimap.remove_by_id(WidgetId::from_u64(999)), None);
    }

    #[test]
    fn clear_removes_all() {
        let mut bimap = WidgetIdBimap::new();
        bimap.insert("a", WidgetId::from_u64(1));
        bimap.insert("b", WidgetId::from_u64(2));
        assert_eq!(bimap.len(), 2);

        bimap.clear();
        assert!(bimap.is_empty());
        assert_eq!(bimap.len(), 0);
        assert_eq!(bimap.get_id("a"), None);
        assert_eq!(bimap.get_id("b"), None);
    }

    #[test]
    fn contains_name_and_id() {
        let mut bimap = WidgetIdBimap::new();
        let id = WidgetId::from_u64(42);
        bimap.insert("btn", id);

        assert!(bimap.contains_name("btn"));
        assert!(!bimap.contains_name("other"));
        assert!(bimap.contains_id(id));
        assert!(!bimap.contains_id(WidgetId::from_u64(99)));
    }

    #[test]
    fn clone_preserves_all_mappings() {
        let mut bimap = WidgetIdBimap::new();
        let id1 = WidgetId::from_u64(1);
        let id2 = WidgetId::from_u64(2);
        bimap.insert("a", id1);
        bimap.insert("b", id2);

        let cloned = bimap.clone();
        assert_eq!(cloned.len(), 2);
        assert_eq!(cloned.get_id("a"), Some(id1));
        assert_eq!(cloned.get_id("b"), Some(id2));
        assert_eq!(cloned.get_name(id1), Some("a"));
        assert_eq!(cloned.get_name(id2), Some("b"));

        // 修改原 bimap 不影响 clone
        bimap.remove_by_name("a");
        assert_eq!(bimap.len(), 1);
        assert_eq!(cloned.len(), 2);
    }
}
