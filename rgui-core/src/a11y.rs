//! 无障碍基础类型：`AccessibilityNode` / `AccessibilityRole` / `AccessibilityAction` / `AccessibilityState`。
//!
//! 平台桥接（AccessKit）不在此处——accesskit 已删除（契约 §4 R1）。此模块仅定义纯 Rust 无障碍类型。

/// 无障碍角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AccessibilityRole {
    /// 未知/默认。
    #[default]
    Unknown,
    /// 按钮。
    Button,
    /// 文本。
    Text,
    /// 容器。
    Container,
}

/// 无障碍动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityAction {
    /// 激活。
    Activate,
    /// 聚焦。
    Focus,
    /// 失焦。
    Blur,
}

/// 无障碍状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccessibilityState {
    /// 是否可聚焦。
    pub focusable: bool,
    /// 是否聚焦中。
    pub focused: bool,
}

/// 无障碍节点。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessibilityNode {
    /// 角色。
    pub role: AccessibilityRole,
    /// 标签。
    pub label: String,
    /// 状态。
    pub state: AccessibilityState,
}

impl AccessibilityNode {
    /// 构造默认节点。
    pub fn new() -> Self {
        Self::default()
    }

    /// 空无障碍节点（对齐 greenfield §B.1）。
    pub fn none() -> Self {
        Self::default()
    }
}
