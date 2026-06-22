//! # rgui-platform
//!
//! rgui 平台抽象——事件类型、焦点管理、命中测试、剪贴板。

pub mod event;
pub mod focus;
pub mod ime;
pub mod router;
pub mod shortcut;
pub mod widget_tree;

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub mod clipboard;

pub use event::{
    CoordinateNormalization, Event, EventSender, Key, Modifiers, MouseButton, MouseEventCoords,
    MouseInputOrigin, NormalizedWindowPoint, logical_window_size_from_physical_size,
    normalize_platform_window_point,
};
pub use focus::FocusManager;
pub use router::EventRouter;
pub use widget_tree::WidgetTree;

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub use clipboard::Clipboard;
