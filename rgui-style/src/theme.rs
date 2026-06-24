//! 主题变量系统——Theme、ThemeVariables、ColorScheme。
//!
//! 定义源自 D4 §5。

use rgui_core::view::PropValue;
use rustc_hash::FxHashMap;
use std::fmt;

// ============================================================================
// ColorScheme
// ============================================================================

/// 色彩方案（D4 §5.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
}

// ============================================================================
// ThemeVariables
// ============================================================================

/// 主题变量集合（D4 §5.2）。
///
/// 存储 CSS 自定义属性（`--variable-name` → PropValue）的映射。
#[derive(Clone, Default)]
pub struct ThemeVariables {
    variables: FxHashMap<String, PropValue>,
}

impl ThemeVariables {
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: FxHashMap::default(),
        }
    }

    /// 插入变量。
    pub fn insert(&mut self, name: impl Into<String>, value: PropValue) {
        self.variables.insert(name.into(), value);
    }

    /// 获取变量值。
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PropValue> {
        self.variables.get(name)
    }

    /// 获取所有变量。
    #[must_use]
    pub fn all(&self) -> &FxHashMap<String, PropValue> {
        &self.variables
    }

    /// 获取所有变量的迭代器。
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PropValue)> {
        self.variables.iter()
    }

    /// 合并另一个变量的值（other 覆盖 self 中同名的）。
    pub fn merge(&mut self, other: &ThemeVariables) {
        for (key, value) in &other.variables {
            self.variables.insert(key.clone(), value.clone());
        }
    }

    /// 变量数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.variables.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }
}

impl fmt::Debug for ThemeVariables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThemeVariables")
            .field("count", &self.variables.len())
            .finish()
    }
}

// ============================================================================
// Theme
// ============================================================================

/// 主题（D4 §5.2）。
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub color_scheme: ColorScheme,
    pub variables: ThemeVariables,
}

impl Theme {
    #[must_use]
    pub fn new(name: impl Into<String>, color_scheme: ColorScheme) -> Self {
        Self {
            name: name.into(),
            color_scheme,
            variables: ThemeVariables::new(),
        }
    }

    /// 获取变量值。
    #[must_use]
    pub fn var(&self, name: &str) -> Option<&PropValue> {
        self.variables.get(name)
    }

    /// 构建默认亮色主题（D4 §5.3）。
    #[must_use]
    pub fn light() -> Self {
        let mut theme = Self::new("rgui-light", ColorScheme::Light);
        let vars = &mut theme.variables;

        // 颜色
        vars.insert("--color-primary", PropValue::str("#3B82F6"));
        vars.insert("--color-primary-hover", PropValue::str("#2563EB"));
        vars.insert("--color-bg", PropValue::str("#FFFFFF"));
        vars.insert("--color-surface", PropValue::str("#F9FAFB"));
        vars.insert("--color-border", PropValue::str("#E5E7EB"));
        vars.insert("--color-text", PropValue::str("#1A1A2E"));
        vars.insert("--color-text-secondary", PropValue::str("#6B7280"));
        vars.insert("--color-danger", PropValue::str("#EF4444"));
        vars.insert("--color-success", PropValue::str("#10B981"));
        vars.insert("--color-warning", PropValue::str("#F59E0B"));

        // 间距
        vars.insert("--spacing-xs", PropValue::str("4px"));
        vars.insert("--spacing-sm", PropValue::str("8px"));
        vars.insert("--spacing-md", PropValue::str("16px"));
        vars.insert("--spacing-lg", PropValue::str("24px"));
        vars.insert("--spacing-xl", PropValue::str("32px"));

        // 圆角
        vars.insert("--radius-sm", PropValue::str("4px"));
        vars.insert("--radius-md", PropValue::str("8px"));
        vars.insert("--radius-lg", PropValue::str("12px"));

        // 字体
        vars.insert("--font-family", PropValue::str("\"Inter\", sans-serif"));
        vars.insert("--font-size-sm", PropValue::str("12px"));
        vars.insert("--font-size-md", PropValue::str("14px"));
        vars.insert("--font-size-lg", PropValue::str("16px"));
        vars.insert("--font-size-xl", PropValue::str("20px"));

        // 阴影
        vars.insert("--shadow-sm", PropValue::str("0 1px 2px rgba(0,0,0,0.05)"));
        vars.insert("--shadow-md", PropValue::str("0 4px 6px rgba(0,0,0,0.1)"));
        vars.insert("--shadow-lg", PropValue::str("0 10px 15px rgba(0,0,0,0.1)"));

        theme
    }

    /// 构建默认暗色主题（D4 §5.3）。
    #[must_use]
    pub fn dark() -> Self {
        let mut theme = Self::new("rgui-dark", ColorScheme::Dark);
        let vars = &mut theme.variables;

        vars.insert("--color-primary", PropValue::str("#60A5FA"));
        vars.insert("--color-primary-hover", PropValue::str("#3B82F6"));
        vars.insert("--color-bg", PropValue::str("#1A1A2E"));
        vars.insert("--color-surface", PropValue::str("#2D2D44"));
        vars.insert("--color-border", PropValue::str("#3F3F5C"));
        vars.insert("--color-text", PropValue::str("#E0E0E0"));
        vars.insert("--color-text-secondary", PropValue::str("#9CA3AF"));
        vars.insert("--color-danger", PropValue::str("#F87171"));
        vars.insert("--color-success", PropValue::str("#34D399"));
        vars.insert("--color-warning", PropValue::str("#FBBF24"));

        vars.insert("--spacing-xs", PropValue::str("4px"));
        vars.insert("--spacing-sm", PropValue::str("8px"));
        vars.insert("--spacing-md", PropValue::str("16px"));
        vars.insert("--spacing-lg", PropValue::str("24px"));
        vars.insert("--spacing-xl", PropValue::str("32px"));

        vars.insert("--radius-sm", PropValue::str("4px"));
        vars.insert("--radius-md", PropValue::str("8px"));
        vars.insert("--radius-lg", PropValue::str("12px"));

        vars.insert("--font-family", PropValue::str("\"Inter\", sans-serif"));
        vars.insert("--font-size-sm", PropValue::str("12px"));
        vars.insert("--font-size-md", PropValue::str("14px"));
        vars.insert("--font-size-lg", PropValue::str("16px"));
        vars.insert("--font-size-xl", PropValue::str("20px"));

        vars.insert("--shadow-sm", PropValue::str("0 1px 2px rgba(0,0,0,0.3)"));
        vars.insert("--shadow-md", PropValue::str("0 4px 6px rgba(0,0,0,0.4)"));
        vars.insert("--shadow-lg", PropValue::str("0 10px 15px rgba(0,0,0,0.5)"));

        theme
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_light_has_variables() {
        let theme = Theme::light();
        assert!(theme.var("--color-primary").is_some());
        assert!(theme.var("--spacing-md").is_some());
    }

    #[test]
    fn theme_dark_has_variables() {
        let theme = Theme::dark();
        assert_eq!(theme.color_scheme, ColorScheme::Dark);
        assert!(theme.variables.len() > 10);
    }

    #[test]
    fn variables_merge() {
        let mut a = ThemeVariables::new();
        a.insert("--color-primary", PropValue::str("red"));

        let mut b = ThemeVariables::new();
        b.insert("--color-primary", PropValue::str("blue"));
        b.insert("--color-bg", PropValue::str("white"));

        a.merge(&b);
        assert_eq!(
            a.get("--color-primary").map(|v| format!("{v:?}")),
            Some(r#"Str("blue")"#.to_string())
        );
        assert!(a.get("--color-bg").is_some());
    }
}
