//! 无障碍树模块（由 `rgui-a11y/tree.rs` 迁入，契约 §4 R1）。

use crate::a11y::AccessibilityNode;
use crate::id::WidgetId;

/// 无障碍树。
#[derive(Debug, Clone, Default)]
pub struct AccessibilityTree {
    /// 根节点。
    pub root: Option<AccessibilityTreeNode>,
}

/// 无障碍树节点。
#[derive(Debug, Clone, Default)]
pub struct AccessibilityTreeNode {
    /// 组件 ID。
    pub widget_id: WidgetId,
    /// 无障碍数据。
    pub node: AccessibilityNode,
    /// 子节点。
    pub children: Vec<AccessibilityTreeNode>,
}

impl AccessibilityTree {
    /// 构造空树。
    pub fn new() -> Self {
        Self::default()
    }
}
