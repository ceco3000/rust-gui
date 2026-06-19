//! `CommandRegistry` — Rhai 命令处理器注册表。
//!
//! 将 `.rgui`/`html!` 中 `on:click="save"` 等命令名称映射到 Rhai 函数调用。
//!
//! ## 设计说明
//!
//! Rhai 脚本中定义的 `fn` 函数存储在编译后的 AST 中，通过 `Engine::call_fn`
//! 按名称调用。本注册表：
//! - 将每次注册的脚本 AST 合并为一个组合 AST
//! - `call_fn` 时通过 `Engine::call_fn` 在组合 AST 中查找并调用函数
//!
//! 依赖 `rhai` 的 `internals` feature（`Variant` trait 和 `call_fn` API 所需）
//! 和 `sync` feature（`Engine: Send + Sync`，支持 `Arc` 共享）。
//!
//! # 示例
//!
//! ```rust
//! use rgui_script::CommandRegistry;
//!
//! let mut registry = CommandRegistry::new();
//! registry.register_script(r#"
//!     fn save() { }
//!     fn delete(id) { print("deleting: " + id); }
//! "#).unwrap();
//!
//! // 调用无参函数
//! registry.call_fn::<()>("save", ()).unwrap();
//!
//! // 调用带参函数
//! registry.call_fn::<()>("delete", ("item-42",)).unwrap();
//! ```

use rhai::{AST, Engine, Scope};
use std::sync::{Arc, Mutex, MutexGuard};

/// 获取 mutex 锁，中毒时恢复内部值。
fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// 命令处理器注册表。
///
/// 内部持有 Rhai 引擎和编译后的脚本 AST。
/// `Arc<Mutex<Engine>>` 设计支持跨线程共享（需 rhai `sync` feature）。
#[derive(Clone)]
pub struct CommandRegistry {
    engine: Arc<Mutex<Engine>>,
    /// 组合 AST（合并所有注册脚本的函数定义）。
    combined_ast: Arc<Mutex<Option<AST>>>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("engine", &"<Engine>")
            .finish()
    }
}

impl CommandRegistry {
    /// 创建空的命令注册表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(Engine::new())),
            combined_ast: Arc::new(Mutex::new(None)),
        }
    }

    /// 注册 Rhai 脚本。
    ///
    /// 脚本中定义的 `fn` 函数自动成为可调用的命令处理器。
    /// 脚本中的顶层语句也会执行（用于初始化全局变量等）。
    ///
    /// 多次调用此方法会累积函数定义——后注册的脚本可调用先注册的函数。
    ///
    /// # Errors
    ///
    /// 脚本编译或执行失败时返回错误。
    pub fn register_script(&mut self, script: &str) -> Result<(), Box<rhai::EvalAltResult>> {
        let engine = lock_mutex(&self.engine);
        let new_ast = engine.compile(script)?;

        // 执行脚本以初始化全局状态
        engine.run_ast_with_scope(&mut Scope::new(), &new_ast)?;
        drop(engine);

        // 合并到组合 AST
        let mut combined = lock_mutex(&self.combined_ast);
        *combined = Some(match combined.take() {
            Some(existing) => existing.merge(&new_ast),
            None => new_ast,
        });
        drop(combined);

        Ok(())
    }

    /// 调用已注册的命令函数。
    ///
    /// - `fn_name`: 函数名（对应 `.rgui` 中 `on:click="fn_name"`）
    /// - `args`: 函数参数元组（实现 `rhai::FuncArgs`）
    ///
    /// # Type Parameters
    ///
    /// - `T`: 返回值类型（无返回值时用 `()`)；必须实现 `rhai::Variant + Clone`
    ///
    /// # Errors
    ///
    /// 函数未注册或调用失败时返回错误。
    pub fn call_fn<T: rhai::Variant + Clone>(
        &mut self,
        fn_name: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<T, Box<rhai::EvalAltResult>> {
        let engine = lock_mutex(&self.engine);
        let mut scope = Scope::new();

        let combined = lock_mutex(&self.combined_ast);
        combined.as_ref().map_or_else(
            || {
                Err(Box::new(rhai::EvalAltResult::ErrorFunctionNotFound(
                    fn_name.to_string(),
                    rhai::Position::NONE,
                )))
            },
            |ast| engine.call_fn(&mut scope, ast, fn_name, args),
        )
    }

    /// 调用已注册的命令函数，返回动态值。
    ///
    /// # Errors
    ///
    /// 函数未注册或调用失败时返回错误。
    pub fn call_fn_dynamic(
        &mut self,
        fn_name: &str,
    ) -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        self.call_fn::<rhai::Dynamic>(fn_name, ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_registry() {
        let registry = CommandRegistry::new();
        let _ = registry;
    }

    #[test]
    fn default_creates_empty_registry() {
        let registry = CommandRegistry::default();
        let _ = registry;
    }

    #[test]
    fn register_empty_script_succeeds() {
        let mut registry = CommandRegistry::new();
        let result = registry.register_script("");
        assert!(result.is_ok());
    }

    #[test]
    fn register_and_call_no_arg_function() {
        let mut registry = CommandRegistry::new();
        registry.register_script("fn save() {}").unwrap();

        let result = registry.call_fn::<()>("save", ());
        assert!(result.is_ok(), "call_fn should succeed: {:?}", result);
    }

    #[test]
    fn register_and_call_function_with_args() {
        let mut registry = CommandRegistry::new();
        registry.register_script("fn add(x, y) { x + y }").unwrap();

        let result: i64 = registry.call_fn("add", (3_i64, 4_i64)).unwrap();
        assert_eq!(result, 7);
    }

    #[test]
    fn register_and_call_function_returning_string() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script(r#"fn greet(name) { "Hello, " + name + "!" }"#)
            .unwrap();

        let result: String = registry.call_fn("greet", ("World",)).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn call_unregistered_function_returns_error() {
        let mut registry = CommandRegistry::new();

        let result = registry.call_fn::<()>("nonexistent", ());
        assert!(result.is_err(), "calling unregistered function should fail");
    }

    #[test]
    fn register_multiple_functions_in_one_script() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script("fn foo() { 1 } fn bar() { 2 }")
            .unwrap();

        let r1: i64 = registry.call_fn("foo", ()).unwrap();
        let r2: i64 = registry.call_fn("bar", ()).unwrap();
        assert_eq!(r1, 1);
        assert_eq!(r2, 2);
    }

    #[test]
    fn register_script_with_syntax_error_returns_error() {
        let mut registry = CommandRegistry::new();
        let result = registry.register_script("fn broken( {");
        assert!(result.is_err());
    }

    #[test]
    fn register_and_call_function_with_bool_param() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script("fn is_enabled(flag) { flag }")
            .unwrap();

        let result: bool = registry.call_fn("is_enabled", (true,)).unwrap();
        assert!(result);

        let result: bool = registry.call_fn("is_enabled", (false,)).unwrap();
        assert!(!result);
    }

    #[test]
    fn call_fn_dynamic_returns_dynamic_value() {
        let mut registry = CommandRegistry::new();
        registry.register_script("fn get_value() { 42 }").unwrap();

        let result = registry.call_fn_dynamic("get_value").unwrap();
        assert_eq!(result.as_int().unwrap(), 42);
    }

    #[test]
    fn registry_is_cloneable() {
        let mut registry = CommandRegistry::new();
        registry.register_script(r#"fn say_hi() { "hi" }"#).unwrap();

        let mut cloned = registry.clone();
        let result: String = cloned.call_fn("say_hi", ()).unwrap();
        assert_eq!(result, "hi");
    }

    #[test]
    fn register_and_call_function_with_float_args() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script("fn multiply(a, b) { a * b }")
            .unwrap();

        let result: f64 = registry.call_fn("multiply", (3.5_f64, 2.0_f64)).unwrap();
        assert!((result - 7.0).abs() < f64::EPSILON);
    }

    #[test]
    fn consecutive_registrations_preserve_functions() {
        let mut registry = CommandRegistry::new();
        registry.register_script("fn first() { 1 }").unwrap();
        registry.register_script("fn second() { 2 }").unwrap();

        let r1: i64 = registry.call_fn("first", ()).unwrap();
        let r2: i64 = registry.call_fn("second", ()).unwrap();
        assert_eq!(r1, 1);
        assert_eq!(r2, 2);
    }
}
