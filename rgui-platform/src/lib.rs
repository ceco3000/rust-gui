//! # rgui-platform
//!
//! rgui 平台层——窗口/输入/IME/焦点（winit 隔离，契约 §1.3）。
//!
//! ## 设计约束
//!
//! - winit 重型平台依赖隔离在此 crate。
//! - 焦点管理原生在此：`FocusManager` / `InputModality`（契约 §4 R1 确认焦点管理本就属 platform，不来自 a11y）。
//! - 可依赖 `rgui-core`；反向禁止。
//!
//! D3 阶段 0：仅建立模块骨架 + 类型占位，不引入 winit 依赖。

pub mod event_loop;
pub mod focus;
pub mod ime;
pub mod input;
pub mod window;

// 焦点管理/输入模态契约导出（§4 R1 定稿：保留原样）
pub use focus::FocusManager;
pub use input::InputModality;

// 窗口/事件循环公共 API（D8 收敛进 platform）
pub use event_loop::{
    run_as, run_as_with_config, App as AppRunner, ControlFlow, EventLoopError, WindowEvent,
};
pub use window::{attributes, Window, WindowConfig};
