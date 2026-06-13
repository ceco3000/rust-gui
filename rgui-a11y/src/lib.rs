//! # rgui-a11y
//!
//! rgui 无障碍系统——AccessibilityTree、焦点管理。
//!
//! 本 crate 基于 AccessKit 提供跨平台无障碍支持。

pub mod tree;
pub mod backend;

pub use tree::AccessibilityTree;
pub use backend::A11yBackend;
