//! 标识符类型：`WidgetId` / `NodeHandle` / `WindowId`。
//!
//! D3 阶段 0：新类型包装。稳定性与分配器在实现阶段补全。

/// 组件 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct WidgetId(pub u64);

impl WidgetId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// 节点句柄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle(pub u64);

impl NodeHandle {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// 窗口 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

impl WindowId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}
