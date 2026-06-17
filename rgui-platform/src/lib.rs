//! # rgui-platform
//!
//! rgui 平台抽象——事件类型、焦点管理、命中测试、剪贴板。

pub mod event;
pub mod focus;
pub mod hit_test;
pub mod ime;
pub mod router;
pub mod shortcut;
pub mod widget_tree;

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub mod clipboard;

pub use event::{Event, EventSender, Key, Modifiers, MouseButton};
pub use focus::FocusManager;
pub use hit_test::HitTester;
pub use router::EventRouter;
pub use widget_tree::WidgetTree;

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub use clipboard::Clipboard;
