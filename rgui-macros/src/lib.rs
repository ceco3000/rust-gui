//! # rgui-macros
//!
//! rgui 过程宏 crate —— **独立 proc-macro crate**（契约 §1.1 硬约束：proc-macro 必须独立）。
//!
//! 提供：
//! - `#[derive(WidgetSpec)]`
//! - `#[derive(AppMessage)]`
//! - `#[derive(PersistState)]`
//! - `html!` 宏
//!
//! > **proc-macro 硬约束**：`#[proc_macro]`/`#[proc_macro_derive]` 函数必须位于 crate 根，
//! > 且 proc-macro crate 不能 `pub use` 非宏项。因此四个宏入口全部定义在本文件（根），
//! > 模块骨架（widget_spec/app_message/persist_state/html）通过子模块组织实现逻辑，
//! > 由根文件调用（D3 阶段 0 为占位透传，未拆分实现模块）。
//!
//! D3 阶段 0：proc-macro 入口骨架 + 契约占位。宏展开逻辑在实现阶段补全（届时引入 syn/quote）。
//! 当前 derive 宏**透传输入**、`html!` **透传输入**，不做任何展开，避免展开期 panic。

use proc_macro::TokenStream;

/// `#[derive(WidgetSpec)]` 宏入口。当前透传输入（D3 占位）。
#[proc_macro_derive(WidgetSpec)]
pub fn widget_spec(input: TokenStream) -> TokenStream {
    input
}

/// `#[derive(AppMessage)]` 宏入口。当前透传输入（D3 占位）。
#[proc_macro_derive(AppMessage)]
pub fn app_message(input: TokenStream) -> TokenStream {
    input
}

/// `#[derive(PersistState)]` 宏入口。当前透传输入（D3 占位）。
#[proc_macro_derive(PersistState)]
pub fn persist_state(input: TokenStream) -> TokenStream {
    input
}

/// `html!` 宏入口（契约 §4 R2 保留：属 Rust 原生构建 DSL，服务 Tier 1 WidgetSpec）。
/// 当前透传输入（D3 占位）。
#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    input
}
