//! # rgui-script
//!
//! rgui Rhai 脚本引擎集成——提供类型注册、函数注册和脚本执行上下文。
//!
//! ## 设计约束
//!
//! - 依赖 `rgui-core`（类型系统）
//! - 依赖 `rhai`（嵌入式脚本引擎）
//! - 纯 Rust，无平台依赖
//!
//! ## 示例
//!
//! ```rust,no_run
//! use rgui_script::ScriptEngine;
//!
//! let mut engine = ScriptEngine::new();
//! engine.engine_mut().register_fn("add", |a: i64, b: i64| a + b);
//! let result: i64 = engine.eval_as("add(1, 2)").unwrap();
//! assert_eq!(result, 3);
//! ```

mod command;
mod engine;
pub mod paint_primitives;
mod prop_registry;

pub use command::CommandRegistry;
pub use engine::ScriptEngine;
pub use prop_registry::PropRegistry;
