//! `ScriptEngine` — Rhai 引擎包装。

use rhai::{Dynamic, Engine, Scope};

/// Rhai 脚本引擎包装。
///
/// 提供类型注册、函数注册和脚本执行能力。
///
/// # 示例
///
/// ```rust
/// use rgui_script::ScriptEngine;
///
/// let mut engine = ScriptEngine::new();
/// engine.engine_mut().register_fn("double", |x: i64| x * 2);
/// let result: i64 = engine.eval_as("double(21)").unwrap();
/// assert_eq!(result, 42);
/// ```
#[derive(Debug)]
pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    /// 创建新的脚本引擎实例。
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    /// 获取内部 Rhai 引擎的不可变引用。
    #[must_use]
    pub const fn engine(&self) -> &Engine {
        &self.engine
    }

    /// 获取内部 Rhai 引擎的可变引用。
    #[must_use]
    pub const fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// 注册自定义类型，使其可在 Rhai 脚本中使用。
    ///
    /// 类型必须实现 `Clone`。
    pub fn register_type<T: rhai::CustomType + Clone>(&mut self) {
        self.engine.register_type::<T>();
    }

    /// 求值 Rhai 表达式并返回动态值。
    ///
    /// # Errors
    ///
    /// 解析失败时返回 `rhai::EvalAltResult`。
    pub fn eval(&mut self, expr: &str) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        self.engine.eval::<Dynamic>(expr)
    }

    /// 求值 Rhai 表达式并转换目标类型。
    ///
    /// # Errors
    ///
    /// 解析或类型转换失败时返回 `rhai::EvalAltResult`。
    pub fn eval_as<T: 'static>(&mut self, expr: &str) -> Result<T, Box<rhai::EvalAltResult>> {
        let value: Dynamic = self.engine.eval(expr)?;
        value
            .try_cast::<T>()
            .ok_or_else(|| format!("type mismatch: expected {}", std::any::type_name::<T>()).into())
    }

    /// 执行 Rhai 脚本（无返回值）。
    ///
    /// # Errors
    ///
    /// 脚本编译或运行时出错时返回 `rhai::EvalAltResult`。
    pub fn run(&mut self, script: &str) -> Result<(), Box<rhai::EvalAltResult>> {
        self.engine.run(script)
    }

    /// 执行 Rhai 脚本并带作用域。
    ///
    /// 作用域在脚本执行后保留变量，可供后续调用使用。
    ///
    /// # Errors
    ///
    /// 脚本编译或运行时出错时返回 `rhai::EvalAltResult`。
    pub fn run_with_scope(
        &mut self,
        scope: &mut Scope,
        script: &str,
    ) -> Result<(), Box<rhai::EvalAltResult>> {
        self.engine.run_with_scope(scope, script)
    }

    /// 在作用域中求值表达式并返回动态值。
    ///
    /// # Errors
    ///
    /// 解析失败时返回 `rhai::EvalAltResult`。
    pub fn eval_with_scope(
        &mut self,
        scope: &mut Scope,
        expr: &str,
    ) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        self.engine.eval_with_scope::<Dynamic>(scope, expr)
    }

    /// 创建新的作用域。
    #[must_use]
    pub fn new_scope(&self) -> Scope<'_> {
        Scope::new()
    }

    /// 注册全局模块。
    pub fn register_module(&mut self, module: rhai::Module) {
        self.engine.register_global_module(module.into());
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_engine_creates_empty_engine() {
        let engine = ScriptEngine::new();
        let _ = engine;
    }

    #[test]
    fn eval_integer_expression() {
        let mut engine = ScriptEngine::new();
        let result: i64 = engine.eval_as("1 + 1").unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn eval_float_expression() {
        let mut engine = ScriptEngine::new();
        let result: f64 = engine.eval_as("3.14 * 2.0").unwrap();
        assert!((result - 6.28).abs() < f64::EPSILON);
    }

    #[test]
    fn eval_bool_expression() {
        let mut engine = ScriptEngine::new();
        let result: bool = engine.eval_as("true && false").unwrap();
        assert!(!result);
    }

    #[test]
    fn eval_string_expression() {
        let mut engine = ScriptEngine::new();
        let result: String = engine.eval_as(r#""hello""#).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn eval_error_on_invalid_syntax() {
        let mut engine = ScriptEngine::new();
        let result = engine.eval("1 + ");
        assert!(result.is_err());
    }

    #[test]
    fn eval_error_on_type_mismatch() {
        let mut engine = ScriptEngine::new();
        // eval_as with wrong type should fail
        let result = engine.eval_as::<bool>("42");
        assert!(result.is_err());
    }

    #[test]
    fn run_script_without_return_value() {
        let mut engine = ScriptEngine::new();
        let result = engine.run("let x = 42;");
        assert!(result.is_ok());
    }

    #[test]
    fn run_with_scope_preserves_variables() {
        let mut engine = ScriptEngine::new();
        let mut scope = Scope::new();

        // Set a variable in the scope before running
        scope.push("x", 42_i64);
        // Use `x = x + 1` (no `let`) to modify the scope variable
        engine.run_with_scope(&mut scope, "x = x + 1;").unwrap();

        // Verify the variable was updated via get_value
        let x: i64 = scope.get_value::<i64>("x").unwrap();
        assert_eq!(x, 43);
    }

    #[test]
    fn eval_with_scope_returns_result() {
        let mut engine = ScriptEngine::new();
        let mut scope = Scope::new();
        scope.push("base", 100_i64);

        let value = engine.eval_with_scope(&mut scope, "base * 2").unwrap();
        let result: i64 = value.try_cast().unwrap();
        assert_eq!(result, 200);
    }

    #[test]
    fn register_custom_type() {
        #[derive(Debug, Clone)]
        #[allow(dead_code)]
        struct Person {
            name: String,
            age: i64,
        }

        impl rhai::CustomType for Person {
            fn build(_builder: rhai::TypeBuilder<'_, Self>) {}
        }

        let mut engine = ScriptEngine::new();
        engine.register_type::<Person>();

        // Verify the type is registered (no panic)
        let _ = engine;
    }

    #[test]
    fn register_fn_via_raw_engine() {
        let mut engine = ScriptEngine::new();
        engine.engine_mut().register_fn("double", |x: i64| x * 2);

        let result: i64 = engine.eval_as("double(21)").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn default_engine_works() {
        let mut engine = ScriptEngine::default();
        let result: i64 = engine.eval_as("1 + 1").unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn new_scope_creates_empty_scope() {
        let engine = ScriptEngine::new();
        let scope = engine.new_scope();
        // Empty scope — get_value for nonexistent key returns None
        let result: Option<i64> = scope.get_value("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn engine_accessor_returns_same_engine() {
        let engine = ScriptEngine::new();
        let _eng: &Engine = engine.engine();
    }

    #[test]
    fn engine_mut_accessor_allows_mutation() {
        let mut engine = ScriptEngine::new();
        engine.engine_mut().register_fn("triple", |x: i64| x * 3);
        let result: i64 = engine.eval_as("triple(5)").unwrap();
        assert_eq!(result, 15);
    }

    #[test]
    fn eval_as_int_with_expression() {
        let mut engine = ScriptEngine::new();
        let result: i64 = engine.eval_as("(1 + 2) * 3 - 4").unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn eval_as_string_concatenation() {
        let mut engine = ScriptEngine::new();
        let result: String = engine.eval_as(r#""Hello, " + "World!""#).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn eval_returns_dynamic() {
        let mut engine = ScriptEngine::new();
        let value = engine.eval("42").unwrap();
        assert_eq!(value.to_string(), "42");
        assert!(value.is::<i64>());
    }
}
