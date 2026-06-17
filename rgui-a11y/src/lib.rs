//! # rgui-a11y
//!
//! rgui 无障碍系统——AccessibilityTree、焦点管理。
//!
//! 本 crate 基于 AccessKit 提供跨平台无障碍支持。

#[cfg(feature = "accesskit")]
pub mod backend;
pub mod tree;

#[cfg(feature = "accesskit")]
pub use backend::{
    AccessKitBackend, AccessibilityBackend, ActionHandlerCallback, NullBackend,
    from_accesskit_action, to_accesskit_role,
};
pub use tree::AccessibilityTree;
