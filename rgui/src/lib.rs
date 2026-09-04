//! # rgui
//!
//! rgui 顶层门面（facade）——瘦身版：核心类型重导出 + 启动/事件循环/渲染协调/交互协调（契约 §1.3 / §3）。
//!
//! ## 职责
//!
//! - 纯重导出收敛（`pub use rgui_core::*` 等，保持使用方兼容）。
//! - `app.rs` 拆分（契约 §3）：启动协调 / AppConfig / App 骨架留在 `app.rs`；
//!   事件循环、渲染协调、交互命中、测试自动化桩、props 同步各自成模块。
//!
//! ## 设计决议（契约 §5）
//!
//! - 删除 `pub use rgui_a11y::*`（a11y 已并入 core，见 core crate）。
//! - 删除 `pub use rgui_components/layout/screen::*`（已并入 core）。
//! - 统一 Tier 1 WidgetSpec；`.rgui`/`.rhai` 声明式路径废弃。
//!
//! D3 阶段 0：模块骨架 + 契约占位，不实现启动/渲染/交互业务逻辑。

pub mod app;
pub mod automation;
pub mod error_boundary;
pub mod event_loop;
pub mod interaction;
pub mod interactive;
pub mod logging;
pub mod paint_factory;
pub mod props_sync;
pub mod render;
pub mod render_coord;
pub mod widget_node;

pub use app::{App, AppConfig};

// 收敛重导出（契约 §1.4）
pub use rgui_core::*;
pub use rgui_platform::{FocusManager, InputModality};
pub use rgui_render::{GlyphKey, PathTessellation};
// derive 宏（macro namespace）与同名 trait（type namespace）可共存
pub use rgui_macros::{html, AppMessage, PersistState, WidgetSpec};
