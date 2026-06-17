//! # rgui-devtools
//!
//! rgui 开发工具——文件监控、热重载、快速重启、双进程通信。
//!
//! 本 crate 提供开发时的热重载和快速重启能力，共 4 层反馈闭环
//!（详见 [D7 §1]）：
//!
//! - **第 1 层**：样式热重载（`.rgss` 文件，< 200ms）——通过
//!   `rgui_style::StyleHotReload` 实现（需启用 `hot-reload` feature）
//! - **第 2 层**：结构热重载（`.rgui` 文件，< 1s）——阶段 2 完善
//! - **第 3 层**：Rust 逻辑反馈（快速重启，2-5s）——阶段 2 完善
//! - **第 4 层**：脚本热重载（`.rhai` 文件，< 500ms）——阶段 2 预留
//!
//! ## 模块结构
//!
//! | 模块 | 功能 |
//! |------|------|
//! | [`config`] | 热重载配置——监控目录、debounce、启用层级 |
//! | [`error`] | 错误类型——`WatchFailed`、`ReloadFailed` 等 |
//! | [`ipc`] | IPC 协议——DisplayProcess / AppProcess 消息（阶段 2 预留） |
//! | [`watcher`] | 文件变更监控——`notify` 封装 + debounce 合并 |
//!
//! ## 快速开始
//!
//! ```ignore
//! use rgui_devtools::{
//!     config::HotReloadConfig,
//!     watcher::{FileWatcher, FileChangeKind},
//! };
//!
//! let config = HotReloadConfig::default();
//! let mut watcher = FileWatcher::new(&config)?;
//!
//! loop {
//!     for change in watcher.check_changes() {
//!         match change.kind {
//!             FileChangeKind::Style => { /* 触发样式重载 */ }
//!             FileChangeKind::Structure => { /* 触发结构重载 */ }
//!             FileChangeKind::Rust => { /* 触发快速重启 */ }
//!             FileChangeKind::Other => { /* 资源文件变更 */ }
//!         }
//!     }
//!     std::thread::sleep(std::time::Duration::from_millis(50));
//! }
//! # Ok::<(), rgui_devtools::error::DevToolsError>(())
//! ```
//!
//! [D7 §1]: https://github.com/rust-gui/rgui/blob/main/docs/D7-开发反馈系统设计.md

pub mod config;
pub mod error;
pub mod fast_restart;
pub mod ipc;
pub mod watcher;
