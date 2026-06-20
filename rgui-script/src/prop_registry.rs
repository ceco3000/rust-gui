//! `PropRegistry` — Rhai↔WidgetView prop 桥接注册表。
//!
//! 响应式状态桥接的核心数据结构（D8 §9.16b RS01）：
//! - Rhai 脚本通过 `set_prop` 写入待更新 prop
//! - 渲染线程通过 `drain()` 获取并注入 `WidgetView`
//!
//! ## 设计约束（D0 §6.2）
//!
//! - `Arc<RwLock<HashMap>>` 实现线程安全共享
//! - `set` 按 widget 合并 prop（同 key 覆盖）
//! - `drain` 清空并返回全部待写入，避免重复注入
//!
//! # 示例
//!
//! ```rust,no_run
//! use rgui_core::id::WidgetId;
//! use rgui_core::view::PropValue;
//! use rgui_script::PropRegistry;
//!
//! let registry = PropRegistry::new();
//! let id = WidgetId::from_u64(1);
//! registry.set(id, "expanded".into(), PropValue::bool(true));
//! assert_eq!(registry.get(id, "expanded"), Some(PropValue::bool(true)));
//! ```

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

use rgui_core::id::WidgetId;
use rgui_core::view::PropValue;

/// 响应式 prop 桥接注册表。
///
/// 线程安全，支持从 Rhai 引擎和渲染线程并发访问。
/// 内部使用 `Arc<RwLock<HashMap>>`——`set` 获取写锁，
/// `get` 获取读锁，`drain` 获取写锁后原子交换。
#[derive(Clone, Default)]
pub struct PropRegistry {
    inner: Arc<RwLock<HashMap<WidgetId, BTreeMap<String, PropValue>>>>,
}

impl PropRegistry {
    /// 创建一个空的 `PropRegistry`。
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 设置 widget 的 prop 值。同 key 覆盖。
    ///
    /// # 参数
    ///
    /// - `id`: 目标 widget 的 `WidgetId`
    /// - `key`: prop 名称
    /// - `value`: prop 值（`PropValue`）
    pub fn set(&self, id: WidgetId, key: String, value: PropValue) {
        let mut guard = self.inner.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.entry(id).or_default().insert(key, value);
    }

    /// 获取 widget 的 prop 值。
    #[must_use]
    pub fn get(&self, id: WidgetId, key: &str) -> Option<PropValue> {
        self.inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&id)
            .and_then(|props| props.get(key).cloned())
    }

    /// 清空并返回全部待写入 prop。
    ///
    /// 渲染线程每帧调用一次，获取 Rhai 脚本写入的全部待更新 prop。
    #[must_use]
    pub fn drain(&self) -> HashMap<WidgetId, BTreeMap<String, PropValue>> {
        let mut guard = self.inner.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *guard)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_registry() {
        let registry = PropRegistry::new();
        let drained = registry.drain();
        assert!(drained.is_empty());
    }

    #[test]
    fn set_and_get_roundtrip() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(1);
        registry.set(id, "label".into(), PropValue::str("hello"));

        let result = registry.get(id, "label");
        assert_eq!(result, Some(PropValue::str("hello")));
    }

    #[test]
    fn get_nonexistent_key_returns_none() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(1);
        assert_eq!(registry.get(id, "bogus"), None);
    }

    #[test]
    fn get_nonexistent_widget_returns_none() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(999);
        assert_eq!(registry.get(id, "anything"), None);
    }

    #[test]
    fn drain_clears_and_returns_all() {
        let registry = PropRegistry::new();
        let id1 = WidgetId::from_u64(1);
        let id2 = WidgetId::from_u64(2);

        registry.set(id1, "expanded".into(), PropValue::bool(true));
        registry.set(id1, "label".into(), PropValue::str("Section 1"));
        registry.set(id2, "color".into(), PropValue::str("blue"));

        let drained = registry.drain();
        assert_eq!(drained.len(), 2);

        let props1 = drained.get(&id1).unwrap();
        assert_eq!(props1.get("expanded"), Some(&PropValue::bool(true)));
        assert_eq!(props1.get("label"), Some(&PropValue::str("Section 1")));

        let props2 = drained.get(&id2).unwrap();
        assert_eq!(props2.get("color"), Some(&PropValue::str("blue")));

        // drain 后注册表为空
        let second_drain = registry.drain();
        assert!(second_drain.is_empty());
    }

    #[test]
    fn drain_returns_empty_when_no_pending() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(1);
        registry.set(id, "x".into(), PropValue::int(42));

        // 第一次 drain 清空
        let drained = registry.drain();
        assert_eq!(drained.len(), 1);

        // 第二次 drain 为空
        let drained2 = registry.drain();
        assert!(drained2.is_empty());
    }

    #[test]
    fn set_overwrites_existing_key() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(1);

        registry.set(id, "value".into(), PropValue::int(10));
        registry.set(id, "value".into(), PropValue::int(20));

        let drained = registry.drain();
        let props = drained.get(&id).unwrap();

        // 最后一次 set 的值保留
        assert_eq!(props.get("value"), Some(&PropValue::int(20)));
    }

    #[test]
    fn set_after_drain_writes_fresh() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(1);

        registry.set(id, "a".into(), PropValue::int(1));
        let _ = registry.drain();

        // drain 后再次写入
        registry.set(id, "b".into(), PropValue::int(2));
        let drained = registry.drain();

        let props = drained.get(&id).unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(props.get("b"), Some(&PropValue::int(2)));
    }

    #[test]
    fn multiple_widgets_independent() {
        let registry = PropRegistry::new();
        let a = WidgetId::from_u64(10);
        let b = WidgetId::from_u64(20);

        registry.set(a, "x".into(), PropValue::int(1));
        registry.set(b, "y".into(), PropValue::int(2));

        // a 和 b 各自独立
        assert_eq!(registry.get(a, "x"), Some(PropValue::int(1)));
        assert_eq!(registry.get(b, "y"), Some(PropValue::int(2)));
        assert_eq!(registry.get(a, "y"), None);
        assert_eq!(registry.get(b, "x"), None);
    }

    #[test]
    fn clone_shares_same_inner() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(1);
        registry.set(id, "shared".into(), PropValue::str("cloned"));

        let clone = registry.clone();
        assert_eq!(clone.get(id, "shared"), Some(PropValue::str("cloned")));

        // clone 写入对原 registry 可见
        clone.set(id, "from-clone".into(), PropValue::int(99));
        assert_eq!(registry.get(id, "from-clone"), Some(PropValue::int(99)));
    }

    #[test]
    fn drain_handles_empty_drain_after_multiple_cycles() {
        let registry = PropRegistry::new();
        let id = WidgetId::from_u64(1);

        for i in 0..5 {
            registry.set(id, "cycle".into(), PropValue::int(i));
            let drained = registry.drain();
            assert_eq!(drained.len(), 1);
            // 清空后立即 drain 应为空
            let empty = registry.drain();
            assert!(empty.is_empty());
        }
    }
}
