//! # rgui-platform
//!
//! rgui 平台抽象——事件类型、焦点管理、命中测试。

pub mod event;
pub mod focus;
pub mod hit_test;
pub mod ime;
pub mod router;
pub mod shortcut;

pub use event::{Event, Key, Modifiers, MouseButton};
pub use focus::FocusManager;
pub use hit_test::HitTester;
pub use router::EventRouter;
