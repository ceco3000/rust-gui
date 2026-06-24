//! CSS 变量系统（AC01）——变量表、var() 引用解析。
//!
//! 定义源自 D4 §2.5。
//!
//! `VariableTable` 存储 CSS 自定义属性（`--name: value`），
//! 支持全局（`:root {}`）和组件作用域（`:host {}`）变量，
//! 以及 `var(--name, fallback)` 引用解析。
//!
//! 变量查找顺序：
//! 1. 当前组件作用域（`:host {}`）
//! 2. 全局作用域（`:root {}` + 注入的 ThemeVariables）
//! 3. 回退值（若提供）

use rgui_core::view::PropValue;
use rustc_hash::FxHashMap;

// ============================================================================
// VariableTable
// ============================================================================

/// CSS 变量表（D4 §2.5.2）。
///
/// 存储解析 `.rgss` 文件后提取的 CSS 自定义属性。
/// 全局变量来自 `:root {}` 块；组件作用域变量来自 `:host {}` 或组件级规则块。
#[derive(Clone, Default, Debug)]
pub struct VariableTable {
    /// `:root {}` 全局变量
    pub global: FxHashMap<String, PropValue>,
    /// 组件作用域变量：scope_key → (name → value)
    pub scoped: FxHashMap<String, FxHashMap<String, PropValue>>,
}

impl VariableTable {
    /// 创建空变量表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            global: FxHashMap::default(),
            scoped: FxHashMap::default(),
        }
    }

    /// 插入全局变量。
    pub fn insert_global(&mut self, name: String, value: PropValue) {
        self.global.insert(name, value);
    }

    /// 插入组件作用域变量。
    ///
    /// `scope_key` 标识组件实例（如 widget 类型名或 WidgetId）。
    pub fn insert_scoped(&mut self, scope_key: String, name: String, value: PropValue) {
        self.scoped
            .entry(scope_key)
            .or_default()
            .insert(name, value);
    }

    /// 添加一条变量声明。
    ///
    /// 名字前必须有 `--` 前缀，存储时保留前缀。
    /// `scope_key` 为 `None` 插入全局作用域。
    pub fn insert_declaration(&mut self, scope_key: Option<&str>, name: &str, value: PropValue) {
        match scope_key {
            Some(key) => self.insert_scoped(key.to_string(), name.to_string(), value),
            None => self.insert_global(name.to_string(), value),
        }
    }

    /// 解析变量引用。
    ///
    /// 查找顺序（D4 §2.5.2）：
    /// 1. 组件作用域
    /// 2. 全局作用域
    ///
    /// 返回 `None` 表示未找到（调用方可能使用回退值）。
    #[must_use]
    pub fn resolve(&self, name: &str, scope_key: Option<&str>) -> Option<PropValue> {
        // 变量名保留 -- 前缀（与存储格式一致）
        // 1. 先查找组件作用域
        if let Some(scope) = scope_key {
            if let Some(scoped_vars) = self.scoped.get(scope) {
                if let Some(value) = scoped_vars.get(name) {
                    return Some(value.clone());
                }
            }
        }

        // 2. 再查找全局作用域
        if let Some(value) = self.global.get(name) {
            return Some(value.clone());
        }

        None
    }

    /// 解析变量引用，支持链式 `var()` 引用和回退值。
    ///
    /// 当解析的值本身也是 `var(...)` 时，递归解析直到非 `var()` 值或检测到循环。
    ///
    /// # 参数
    /// * `var_text` — 原始 `var(--name)` 或 `var(--name, fallback)` 文本
    /// * `scope_key` — 当前组件作用域
    ///
    /// 返回 `None` 表示无法解析且无回退值。
    #[must_use]
    pub fn resolve_var_ref(&self, var_text: &str, scope_key: Option<&str>) -> Option<PropValue> {
        let inner = var_text.trim();
        // 去除 `var(` 前缀和 `)`
        let inner = inner
            .strip_prefix("var(")
            .and_then(|s| s.strip_suffix(')'))?;
        let inner = inner.trim();

        // 解析变量名和可选的 fallback
        let (var_name, fallback) = if let Some(comma_pos) = inner.find(',') {
            let name = inner[..comma_pos].trim();
            let fallback = inner[comma_pos + 1..].trim();
            (name, Some(fallback.to_string()))
        } else {
            (inner, None)
        };

        // 解析变量值（递归，最多 10 层，防止循环）
        self.resolve_recursive(var_name, fallback.as_deref(), scope_key, 0)
    }

    /// 递归解析变量，最多 `max_depth` 层。
    fn resolve_recursive(
        &self,
        name: &str,
        fallback: Option<&str>,
        scope_key: Option<&str>,
        depth: usize,
    ) -> Option<PropValue> {
        const MAX_DEPTH: usize = 10;

        if depth >= MAX_DEPTH {
            // 达到最大深度，尝试回退值
            return self.resolve_fallback_value(fallback);
        }

        let resolved = self.resolve(name, scope_key);

        match resolved {
            Some(PropValue::Str(ref s)) if s.starts_with("var(") => {
                // 链式引用：递归解析
                // 为避免无限循环，简单检测：如果 inner name 与当前 name 相同，回退
                let inner_text = s.as_ref();
                let inner = inner_text
                    .strip_prefix("var(")
                    .and_then(|s| s.strip_suffix(')'))?;
                let inner = inner.trim();
                let inner_name = if let Some(comma) = inner.find(',') {
                    inner[..comma].trim()
                } else {
                    inner
                };

                if inner_name == name {
                    // 自引用循环，回退
                    return self.resolve_fallback_value(fallback);
                }

                self.resolve_recursive(inner_name, fallback, scope_key, depth + 1)
            },
            Some(value) => Some(value),
            None => self.resolve_fallback_value(fallback),
        }
    }

    /// 解析回退值。
    fn resolve_fallback_value(&self, fallback: Option<&str>) -> Option<PropValue> {
        fallback.map(|f| {
            let f = f.trim();
            // 回退值可能被引号包裹
            let unquoted = if (f.starts_with('"') && f.ends_with('"'))
                || (f.starts_with('\'') && f.ends_with('\''))
            {
                &f[1..f.len() - 1]
            } else {
                f
            };
            PropValue::Str(std::sync::Arc::from(unquoted))
        })
    }

    /// 从主题注入变量到全局作用域（D4 §2.5.3）。
    ///
    /// ThemeVariables 作为预定义变量集注入到 AC01 变量表的全局作用域。
    /// 已存在的同名变量不会被覆盖（.rgss 中显式定义的变量优先）。
    pub fn inject_theme_variables(&mut self, theme: &crate::theme::Theme) {
        for (name, value) in theme.variables.all() {
            let entry = self.global.entry(name.clone());
            entry.or_insert_with(|| value.clone());
        }
    }
}

// ============================================================================
// 解析 `var()` 引用的辅助函数
// ============================================================================

/// 从属性值字符串中提取 `var()` 引用并返回变量名和回退值。
///
/// 例如：`"var(--wa-spacing, 16px)"` → `Some(("--wa-spacing", Some("16px")))`
///
/// 如果不是 `var()` 引用，返回 `None`。
#[must_use]
pub fn parse_var_reference(value: &str) -> Option<(&str, Option<&str>)> {
    let s = value.trim();
    let inner = s.strip_prefix("var(").and_then(|s| s.strip_suffix(')'))?;
    let inner = inner.trim();

    if let Some(comma_pos) = inner.find(',') {
        let name = inner[..comma_pos].trim();
        let fallback = inner[comma_pos + 1..].trim();
        Some((name, Some(fallback)))
    } else {
        Some((inner, None))
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn variable_table_global_insert_and_resolve() {
        let mut vt = VariableTable::new();
        vt.insert_global("--wa-spacing".into(), PropValue::str("16px"));
        vt.insert_global("--wa-color-primary".into(), PropValue::str("#3B82F6"));

        assert_eq!(
            vt.resolve("--wa-spacing", None),
            Some(PropValue::str("16px"))
        );
        assert_eq!(
            vt.resolve("--wa-color-primary", None),
            Some(PropValue::str("#3B82F6"))
        );
        assert_eq!(vt.resolve("--unknown", None), None);
    }

    #[test]
    fn variable_table_scoped_overrides_global() {
        let mut vt = VariableTable::new();
        vt.insert_global("--wa-spacing".into(), PropValue::str("16px"));
        vt.insert_scoped(
            "Button".into(),
            "--wa-spacing".into(),
            PropValue::str("8px"),
        );

        // Scoped 值优先
        assert_eq!(
            vt.resolve("--wa-spacing", Some("Button")),
            Some(PropValue::str("8px"))
        );
        // 其他 scope 回退到 global
        assert_eq!(
            vt.resolve("--wa-spacing", Some("Label")),
            Some(PropValue::str("16px"))
        );
    }

    #[test]
    fn variable_table_resolve_var_ref_simple() {
        let mut vt = VariableTable::new();
        vt.insert_global("--wa-spacing".into(), PropValue::str("16px"));

        let result = vt.resolve_var_ref("var(--wa-spacing)", None);
        assert_eq!(result, Some(PropValue::str("16px")));
    }

    #[test]
    fn variable_table_resolve_var_ref_with_fallback() {
        let vt = VariableTable::new();

        // 变量不存在，使用回退值
        let result = vt.resolve_var_ref("var(--unknown, 12px)", None);
        assert_eq!(result, Some(PropValue::str("12px")));
    }

    #[test]
    fn variable_table_resolve_var_ref_fallback_when_defined() {
        let mut vt = VariableTable::new();
        vt.insert_global("--wa-spacing".into(), PropValue::str("16px"));

        // 变量存在时忽略回退值
        let result = vt.resolve_var_ref("var(--wa-spacing, 8px)", None);
        assert_eq!(result, Some(PropValue::str("16px")));
    }

    #[test]
    fn variable_table_resolve_var_ref_chained() {
        let mut vt = VariableTable::new();
        vt.insert_global("--a".into(), PropValue::str("var(--b)"));
        vt.insert_global("--b".into(), PropValue::str("var(--c)"));
        vt.insert_global("--c".into(), PropValue::str("#ff0000"));

        let result = vt.resolve_var_ref("var(--a)", None);
        assert_eq!(result, Some(PropValue::str("#ff0000")));
    }

    #[test]
    fn variable_table_resolve_var_ref_self_cycle_uses_fallback() {
        let mut vt = VariableTable::new();
        vt.insert_global("--x".into(), PropValue::str("var(--x)"));

        // 自引用循环，使用回退值
        let result = vt.resolve_var_ref("var(--x, #000)", None);
        assert_eq!(result, Some(PropValue::str("#000")));
    }

    #[test]
    fn variable_table_resolve_var_ref_cycle_no_fallback() {
        let mut vt = VariableTable::new();
        vt.insert_global("--x".into(), PropValue::str("var(--x)"));

        // 自引用循环，无回退值
        let result = vt.resolve_var_ref("var(--x)", None);
        assert_eq!(result, None);
    }

    #[test]
    fn variable_table_inject_theme_preserves_existing() {
        let mut vt = VariableTable::new();
        vt.insert_global("--color-primary".into(), PropValue::str("#CUSTOM"));

        let theme = Theme::light();
        vt.inject_theme_variables(&theme);

        // .rgss 中显式定义的值不被 theme 覆盖
        assert_eq!(
            vt.resolve("--color-primary", None),
            Some(PropValue::str("#CUSTOM"))
        );
        // theme 中没有被覆盖的变量仍然可用
        assert_eq!(
            vt.resolve("--color-bg", None),
            Some(PropValue::str("#FFFFFF"))
        );
    }

    #[test]
    fn variable_table_resolve_var_ref_with_scope() {
        let mut vt = VariableTable::new();
        vt.insert_global("--wa-spacing".into(), PropValue::str("16px"));
        vt.insert_scoped(
            "accordion-item".into(),
            "--wa-spacing".into(),
            PropValue::str("12px"),
        );

        // Scoped 变量优先
        let result = vt.resolve_var_ref("var(--wa-spacing)", Some("accordion-item"));
        assert_eq!(result, Some(PropValue::str("12px")));
    }

    #[test]
    fn parse_var_reference_simple() {
        let (name, fallback) = parse_var_reference("var(--wa-spacing)").unwrap();
        assert_eq!(name, "--wa-spacing");
        assert_eq!(fallback, None);
    }

    #[test]
    fn parse_var_reference_with_fallback() {
        let (name, fallback) = parse_var_reference("var(--wa-spacing, 16px)").unwrap();
        assert_eq!(name, "--wa-spacing");
        assert_eq!(fallback, Some("16px"));
    }

    #[test]
    fn parse_var_reference_not_var() {
        assert!(parse_var_reference("16px").is_none());
    }

    #[test]
    fn insert_declaration_global() {
        let mut vt = VariableTable::new();
        vt.insert_declaration(None, "--wa-spacing", PropValue::str("16px"));

        assert_eq!(
            vt.resolve("--wa-spacing", None),
            Some(PropValue::str("16px"))
        );
    }

    #[test]
    fn insert_declaration_scoped() {
        let mut vt = VariableTable::new();
        vt.insert_declaration(Some("Button"), "--button-padding", PropValue::str("8px"));

        assert_eq!(
            vt.resolve("--button-padding", Some("Button")),
            Some(PropValue::str("8px"))
        );
    }

    #[test]
    fn extract_variables_from_rgss_rules_root() {
        use crate::parser::parse_rgss;

        let source =
            ":root { --wa-spacing: 16px; --wa-color-primary: #3B82F6; }\nButton { color: red; }";

        let rules = parse_rgss(source).unwrap();
        let vt = crate::variable::extract_variables_from_rules(&rules);

        // 全局变量已提取（注意：16px 被解析器转换为 Int(16)）
        assert_eq!(vt.resolve("--wa-spacing", None), Some(PropValue::Int(16)));
        assert_eq!(
            vt.resolve("--wa-color-primary", None),
            Some(PropValue::str("#3B82F6"))
        );
    }

    #[test]
    fn extract_variables_from_rgss_rules_host() {
        use crate::parser::parse_rgss;

        let source = ":host { --button-padding: 8px; }\nButton { color: blue; }";

        let rules = parse_rgss(source).unwrap();
        let vt = crate::variable::extract_variables_from_rules(&rules);

        // :host 作用域变量已提取（scope key = ":host"）
        assert_eq!(
            vt.resolve("--button-padding", Some(":host")),
            Some(PropValue::Int(8))
        );
    }

    #[test]
    fn variable_table_resolve_var_scoped_fallback_to_global() {
        let mut vt = VariableTable::new();
        vt.insert_global("--wa-color".into(), PropValue::str("#000"));
        vt.insert_scoped(
            "button".into(),
            "--wa-spacing".into(),
            PropValue::str("8px"),
        );

        // --wa-color 不在 scoped 中，回退到 global
        assert_eq!(
            vt.resolve("--wa-color", Some("button")),
            Some(PropValue::str("#000"))
        );
        // --wa-spacing 在 scoped 中
        assert_eq!(
            vt.resolve("--wa-spacing", Some("button")),
            Some(PropValue::str("8px"))
        );
    }
}

// ============================================================================
// 从已解析的 StyleRule 列表中提取变量
// ============================================================================

use crate::selector::{Selector, StyleRule};

/// 从已解析的 StyleRule 列表中提取 CSS 变量声明。
///
/// 变量声明规则：
/// - 在 `:root` 选择器内的 `--*` 声明 → 全局变量
/// - 在 `:host` 选择器内的 `--*` 声明 → 组件级（scope key = ":host"）
/// - 其他选择器内的 `--*` 声明 → 被忽略（非标准 .rgss 用法）
///
/// 调用方可以在变量提取后将变量声明从规则中移除（避免重复）。
#[must_use]
pub fn extract_variables_from_rules(rules: &[StyleRule]) -> VariableTable {
    let mut table = VariableTable::new();

    for rule in rules {
        let scope_key = match &rule.selector {
            Selector::Type(t) if t == ":root" => None,
            Selector::Type(t) if t == ":host" => Some(":host"),
            _ => continue, // 只处理 :root 和 :host
        };

        for (prop_name, value) in &rule.declarations {
            if prop_name.starts_with("--") {
                match scope_key {
                    None => {
                        table.insert_global(prop_name.to_string(), value.clone());
                    },
                    Some(sk) => {
                        table.insert_scoped(sk.to_string(), prop_name.to_string(), value.clone());
                    },
                }
            }
        }
    }

    table
}
