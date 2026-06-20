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

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use rhai::{AST, Engine, Scope};

use rgui_core::id::WidgetId;
use rgui_core::view::PropValue;

use crate::prop_registry::PropRegistry;

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
///
/// 同时持有 [`PropRegistry`] 用于 Rhai↔WidgetView prop 桥接，
/// 以及临时 `id_map` 用于字符串→`WidgetId` 映射（RS03 完成后替换）。
#[derive(Clone)]
pub struct CommandRegistry {
    engine: Arc<Mutex<Engine>>,
    /// 组合 AST（合并所有注册脚本的函数定义）。
    combined_ast: Arc<Mutex<Option<AST>>>,
    /// 响应式 prop 桥接注册表（RS01）。
    prop_registry: PropRegistry,
    /// 临时字符串→WidgetId 映射。
    /// RS03 完成后替换为双向映射。
    #[allow(dead_code)]
    id_map: Arc<Mutex<HashMap<String, WidgetId>>>,
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
    ///
    /// 自动向 Rhai 引擎注册 `set_prop(id, key, value)` 和 `get_prop(id, key)` 函数，
    /// 用于 Rhai 脚本读写 `WidgetView` props（RS02）。
    #[must_use]
    pub fn new() -> Self {
        let prop_registry = PropRegistry::new();
        let id_map = Arc::new(Mutex::new(HashMap::new()));
        let mut engine = Engine::new();

        // ── 注册 set_prop(id, key, value) ────────────────────────────
        let pr = prop_registry.clone();
        let im = Arc::clone(&id_map);
        engine.register_fn("set_prop", move |id: &str, key: &str, value: &str| {
            let widget_id = {
                // 使用 WidgetId::new() 分配唯一 ID（全局原子计数器）
                *lock_mutex(&im)
                    .entry(id.to_string())
                    .or_insert_with(WidgetId::new)
            };
            pr.set(widget_id, key.to_string(), PropValue::str(value));
        });

        // ── 注册 get_prop(id, key) -> String ────────────────────────
        let pr2 = prop_registry.clone();
        let im2 = Arc::clone(&id_map);
        engine.register_fn("get_prop", move |id: &str, key: &str| -> String {
            let widget_id = lock_mutex(&im2).get(id).copied();
            widget_id.map_or_else(String::new, |wid| {
                pr2.get(wid, key)
                    .map(|pv| pv.to_string())
                    .unwrap_or_default()
            })
        });

        Self {
            engine: Arc::new(Mutex::new(engine)),
            combined_ast: Arc::new(Mutex::new(None)),
            prop_registry,
            id_map,
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

    /// 获取内部 Rhai 引擎的可变引用（用于注册 Rust 端类型和函数）。
    ///
    /// 锁定内部 `Arc<Mutex<Engine>>`，中毒时恢复内部值。
    ///
    /// # 示例
    ///
    /// ```rust
    /// # use rgui_script::CommandRegistry;
    /// # use rhai::Engine;
    /// let registry = CommandRegistry::new();
    /// registry.engine_mut().register_type::<i64>();
    /// ```
    pub fn engine_mut(&self) -> MutexGuard<'_, Engine> {
        lock_mutex(&self.engine)
    }

    /// 获取内部 [`PropRegistry`] 的引用。
    ///
    /// 用于渲染线程每帧调用 `drain()` 获取 Rhai 脚本写入的待更新 props。
    #[must_use]
    pub const fn prop_registry(&self) -> &PropRegistry {
        &self.prop_registry
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

    #[test]
    fn engine_mut_returns_guard_allowing_type_registration() {
        let registry = CommandRegistry::new();
        // Verify engine_mut() returns a guard that allows type registration
        registry.engine_mut().register_type::<i64>();
        // Registration succeeded without panic
    }

    #[test]
    fn engine_mut_guard_is_released_after_use() {
        let registry = CommandRegistry::new();
        {
            let _guard = registry.engine_mut();
            // guard held, _guard dropped at end of block
        }
        // Can acquire again — guard was released
        registry.engine_mut().register_type::<String>();
    }

    // ── RS02: set_prop/get_prop Rhai 注册 ────────────────────────────

    #[test]
    fn set_prop_via_rhai_script_stores_in_prop_registry() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script(
                r#"
                fn update_label() {
                    set_prop("section1", "label", "Hello World");
                }
                "#,
            )
            .unwrap();

        registry.call_fn::<()>("update_label", ()).unwrap();

        // 验证 PropRegistry 中可见写入的 prop
        let drained = registry.prop_registry().drain();
        assert_eq!(drained.len(), 1, "应有一个 widget 的 props");
        let (_, props) = drained.iter().next().unwrap();
        let val = props.get("label").unwrap();
        assert_eq!(val.to_string(), "\"Hello World\"");
    }

    #[test]
    fn get_prop_via_rhai_script_reads_from_prop_registry() {
        let mut registry = CommandRegistry::new();
        // 先用 set_prop 写入，再用 get_prop 读取
        registry
            .register_script(
                r#"
                fn write_then_read() {
                    set_prop("s1", "expanded", "true");
                    get_prop("s1", "expanded")
                }
                "#,
            )
            .unwrap();

        let result: String = registry.call_fn("write_then_read", ()).unwrap();
        assert_eq!(result, "\"true\"");
    }

    #[test]
    fn get_prop_returns_empty_for_unknown_key() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script(
                r#"
                fn read_unknown() {
                    get_prop("s1", "nonexistent")
                }
                "#,
            )
            .unwrap();

        let result: String = registry.call_fn("read_unknown", ()).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn get_prop_returns_empty_for_unknown_widget() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script(
                r#"
                fn read_unknown_widget() {
                    get_prop("ghost", "anything")
                }
                "#,
            )
            .unwrap();

        let result: String = registry.call_fn("read_unknown_widget", ()).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn set_prop_multiple_keys_per_widget() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script(
                r#"
                fn set_many() {
                    set_prop("w", "label", "Save");
                    set_prop("w", "enabled", "false");
                    set_prop("w", "color", "blue");
                }
                "#,
            )
            .unwrap();

        registry.call_fn::<()>("set_many", ()).unwrap();

        let drained = registry.prop_registry().drain();
        assert_eq!(drained.len(), 1);
        let (_, props) = drained.iter().next().unwrap();
        assert_eq!(props.len(), 3);
        assert_eq!(props.get("label").unwrap().to_string(), "\"Save\"");
        assert_eq!(props.get("enabled").unwrap().to_string(), "\"false\"");
        assert_eq!(props.get("color").unwrap().to_string(), "\"blue\"");
    }

    #[test]
    fn set_prop_multiple_widgets_independent() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script(
                r#"
                fn set_multi() {
                    set_prop("a", "x", "1");
                    set_prop("b", "y", "2");
                }
                "#,
            )
            .unwrap();

        registry.call_fn::<()>("set_multi", ()).unwrap();

        let drained = registry.prop_registry().drain();
        assert_eq!(drained.len(), 2, "两个不同 widget");
    }

    #[test]
    fn set_prop_overwrites_previous_value() {
        let mut registry = CommandRegistry::new();
        registry
            .register_script(
                r#"
                fn overwrite() {
                    set_prop("w", "v", "first");
                    set_prop("w", "v", "second");
                }
                "#,
            )
            .unwrap();

        registry.call_fn::<()>("overwrite", ()).unwrap();

        let drained = registry.prop_registry().drain();
        let (_, props) = drained.iter().next().unwrap();
        assert_eq!(props.get("v").unwrap().to_string(), "\"second\"");
    }

    #[test]
    fn set_prop_with_same_string_id_reuses_widget_id() {
        let mut registry = CommandRegistry::new();
        // 两次调用 set_prop 使用同一个字符串 ID
        registry
            .register_script(
                r#"
                fn two_calls() {
                    set_prop("w1", "first", "a");
                    set_prop("w1", "second", "b");
                }
                "#,
            )
            .unwrap();

        registry.call_fn::<()>("two_calls", ()).unwrap();

        let drained = registry.prop_registry().drain();
        // 同一个 widget ID，应有合并后的 2 个 key
        assert_eq!(drained.len(), 1);
        let (_, props) = drained.iter().next().unwrap();
        assert_eq!(props.len(), 2);
        assert_eq!(props.get("first").unwrap().to_string(), "\"a\"");
        assert_eq!(props.get("second").unwrap().to_string(), "\"b\"");
    }

    #[test]
    fn prop_registry_accessor_returns_valid_reference() {
        let registry = CommandRegistry::new();
        let pr = registry.prop_registry();
        // 初始状态：drain 为空
        assert!(pr.drain().is_empty());
    }
}
