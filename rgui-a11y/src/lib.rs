//! # rgui-a11y
//!
//! rgui 无障碍系统——AccessibilityTree、焦点管理。
//!
//! 本 crate 基于 AccessKit 提供跨平台无障碍支持。

pub mod backend;
pub mod tree;

pub use backend::{AccessKitBackend, AccessibilityBackend, NullBackend, to_accesskit_role};
pub use tree::AccessibilityTree;
