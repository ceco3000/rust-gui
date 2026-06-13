//! Widget 注册表——WidgetRegistry。
//!
//! 管理已注册 WidgetSpec 实现的名称和元数据。
//!
//! ## 设计说明
//!
//! 由于 `WidgetSpec` 关联类型在 trait object 中无法直接表达
//! （`AppMessage: Clone` 导致不 dyn-compatible），
//! 注册表存储名称而非 trait object。
//! 类型级 widget 查找通过 `rgui-macros` 的静态分发实现。
//!
//! 定义源自 D0 §4.1。

use rustc_hash::FxHashSet;
use std::fmt;

/// Widget 注册错误。
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// 同名 widget 已注册。
    #[error("widget '{0}' 已注册，不允许重复注册")]
    DuplicateName(&'static str),
}

/// Widget 注册表。
///
/// 存储所有已注册 widget 的名称。`ui!` 宏在编译时
/// 验证引用的 widget 名称是否在注册表中。
pub struct WidgetRegistry {
    names: FxHashSet<&'static str>,
}

impl WidgetRegistry {
    /// 创建空的注册表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            names: FxHashSet::default(),
        }
    }

    /// 注册一个 widget 名称。
    ///
    /// # 错误
    ///
    /// 如果同名 widget 已注册，返回 [`RegistryError::DuplicateName`]。
    pub fn register(&mut self, name: &'static str) -> Result<(), RegistryError> {
        if !self.names.insert(name) {
            return Err(RegistryError::DuplicateName(name));
        }
        Ok(())
    }

    /// 检查指定名称是否已注册。
    #[must_use]
    pub fn contains(&self, name: &'static str) -> bool {
        self.names.contains(name)
    }

    /// 返回已注册 widget 数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// 注册表是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// 返回所有已注册 widget 名称的迭代器。
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.names.iter().copied()
    }
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for WidgetRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WidgetRegistry")
            .field("count", &self.names.len())
            .finish()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_new_is_empty() {
        let registry = WidgetRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn registry_register_and_contains() {
        let mut registry = WidgetRegistry::new();
        registry.register("Button").unwrap();
        assert!(registry.contains("Button"));
        assert!(!registry.contains("TextField"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn registry_duplicate_errors() {
        let mut registry = WidgetRegistry::new();
        registry.register("Button").unwrap();
        let result = registry.register("Button");
        assert!(result.is_err());
        match result.unwrap_err() {
            RegistryError::DuplicateName("Button") => {},
            other => panic!("期望 DuplicateName，得到 {other:?}"),
        }
    }

    #[test]
    fn registry_multiple_widgets() {
        let mut registry = WidgetRegistry::new();
        registry.register("Button").unwrap();
        registry.register("TextField").unwrap();
        registry.register("DataGrid").unwrap();
        assert_eq!(registry.len(), 3);
        let mut names: Vec<_> = registry.names().collect();
        names.sort();
        assert_eq!(names, vec!["Button", "DataGrid", "TextField"]);
    }

    #[test]
    fn registry_default_is_empty() {
        let registry = WidgetRegistry::default();
        assert_eq!(registry.len(), 0);
    }
}
