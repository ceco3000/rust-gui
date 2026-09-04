//! `#[derive(AppMessage)]` 宏。
//!
//! > **proc-macro 硬约束**：宏入口（`#[proc_macro_derive(AppMessage)]`）定义在 crate 根
//!   `lib.rs`（Rust 要求 proc-macro 函数位于根）。本文件为模块骨架文档占位，
//!   宏展开逻辑（syn/quote）在实现阶段补全。
