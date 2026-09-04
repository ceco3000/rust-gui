//! 组件注册表占位模块。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 注册错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// ID 冲突。
    DuplicateId,
    /// 未找到。
    NotFound,
}

/// 组件注册表（按名称注册 WidgetSpec trait 对象）。
#[derive(Debug, Default)]
pub struct WidgetRegistry {
    inner: RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>,
}

impl WidgetRegistry {
    /// 构造空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册组件。D3 占位，实际注册逻辑在实现阶段补全。
    pub fn register(&self, _name: &str, _widget: Arc<dyn std::any::Any + Send + Sync>) {
        // todo!("组件注册在实现阶段补全")
    }
}
