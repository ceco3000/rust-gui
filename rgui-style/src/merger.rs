//! 样式合并——4 层优先级合并。
//!
//! 定义源自 D4 §6。

use crate::theme::Theme;
use crate::variable::VariableTable;
use rgui_core::view::PropValue;
use rustc_hash::FxHashSet;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// 样式合并器（D4 §6.2）。
///
/// 按优先级从低到高合并样式属性（D4 §6.1 5 级优先级）：
/// 1. 框架默认样式
/// 2. .rgss 匹配的规则
/// 3. 组件 inline style
/// 4. 主题变量解析
/// 5. !important 声明（最高）
pub struct StyleMerger;

impl StyleMerger {
    /// 合并四层样式 + !important 覆盖，返回最终属性集。
    ///
    /// `!important` 标记的属性在任何层级声明后，会覆盖所有非 important 的同名属性。
    /// 高层 important 覆盖低层 important。
    ///
    /// `variable_table` 可选：CSS 变量表（AC01），用于解析 `var()` 引用。
    /// 传入 `None` 时仅使用 Theme 中的主题变量。
    #[must_use]
    pub fn merge(
        default_style: &BTreeMap<&'static str, PropValue>,
        rgss_matched: &BTreeMap<Arc<str>, PropValue>,
        rgss_important: &BTreeSet<Arc<str>>,
        inline_style: &BTreeMap<&'static str, PropValue>,
        inline_important: &BTreeSet<&'static str>,
        theme: &Theme,
        variable_table: Option<&VariableTable>,
    ) -> BTreeMap<Arc<str>, PropValue> {
        let mut result = BTreeMap::new();

        // ================================================================
        // 阶段 1：合并非 !important 属性（优先级 1→4）
        // ================================================================

        // 第 1 层：框架默认样式
        for (&key, value) in default_style {
            result.insert(Arc::from(key), value.clone());
        }

        // 第 2 层：.rgss 匹配（非 important）
        for (key, value) in rgss_matched {
            if !rgss_important.contains(key) {
                result.insert(Arc::clone(key), value.clone());
            }
        }

        // 第 3 层：inline style（非 important）
        for (&key, value) in inline_style {
            if !inline_important.contains(key) {
                result.insert(Arc::from(key), value.clone());
            }
        }

        // 第 4 层：解析 var() 引用（非 important）
        // 优先使用 VariableTable (AC01 CSS 变量)，回退到 Theme
        let var_prefix = "var(";
        let keys: Vec<Arc<str>> = result.keys().cloned().collect();
        for key in keys {
            if let Some(PropValue::Str(ref s)) = result.get(&key) {
                if s.starts_with(var_prefix) {
                    // 尝试通过 VariableTable 解析（含 fallback 支持）
                    let resolved = variable_table.and_then(|vt| vt.resolve_var_ref(s, None));
                    if let Some(value) = resolved {
                        result.insert(key, value);
                    } else {
                        // 回退到 Theme 变量解析
                        let var_name = s
                            .trim_start_matches(var_prefix)
                            .trim_end_matches(')')
                            .trim();
                        let mut visited = FxHashSet::default();
                        if let Some(resolved) =
                            Self::resolve_var_recursive(var_name, theme, &mut visited)
                        {
                            result.insert(key, resolved);
                        }
                    }
                    // 两者都失败：保留原始引用（降级）
                }
            }
        }

        // ================================================================
        // 阶段 2：叠加 !important 属性（优先级 5，D4 §6.1）
        // ================================================================
        // 高层 important 覆盖低层 important

        // 第 5a 层：default 层 !important（default 目前不支持，预留）
        // 大多数场景中框架默认不使用 !important，跳过。

        // 第 5b 层：.rgss 匹配的重要属性
        for prop_name in rgss_important {
            if let Some(value) = rgss_matched.get(prop_name) {
                result.insert(Arc::clone(prop_name), value.clone());
            }
        }

        // 第 5c 层：inline style 的重要属性
        for &prop_name in inline_important {
            if let Some(value) = inline_style.get(prop_name) {
                result.insert(Arc::from(prop_name), value.clone());
            }
        }

        result
    }

    // ========================================================================
    // 变量解析辅助方法
    // ========================================================================

    /// 递归解析 CSS 变量引用，检测循环（D4 §10）。
    ///
    /// 遍历主题变量链（`--a: var(--b)` → `--b: var(--c)` → …），直到找到非 `var()` 的值。
    /// 如果检测到循环引用（变量名已在 `visited` 中），返回 `None`，
    /// 调用方应保留原始 `var()` 引用作为降级处理。
    ///
    /// # 参数
    /// * `name` — 要解析的变量名（不含 `--` 前缀的键已在调用方处理）
    /// * `theme` — 主题变量集合
    /// * `visited` — 当前解析链中已访问的变量名集合（用于循环检测）
    #[must_use]
    fn resolve_var_recursive(
        name: &str,
        theme: &Theme,
        visited: &mut FxHashSet<String>,
    ) -> Option<PropValue> {
        // 循环检测：如果变量已在当前解析链中，检测到循环
        if !visited.insert(name.to_string()) {
            return None;
        }

        // 查找变量值
        let value = theme.var(name)?;

        match value {
            PropValue::Str(s) if s.starts_with("var(") => {
                // 链式引用：提取内部变量名，递归解析
                let nested_name = s.trim_start_matches("var(").trim_end_matches(')').trim();
                let result = Self::resolve_var_recursive(nested_name, theme, visited);
                // 递归返回后移除当前变量名（回溯，允许不同路径复用）
                visited.remove(name);
                result
            },
            other => {
                // 非 var() 值：找到具体值
                visited.remove(name);
                Some(other.clone())
            },
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ColorScheme, Theme};

    /// Helper: empty BTreeMap<Arc<str>, PropValue>
    fn empty_rgss_map() -> BTreeMap<Arc<str>, PropValue> {
        BTreeMap::new()
    }
    /// Helper: empty BTreeMap<&'static str, PropValue>
    fn empty_inline_map() -> BTreeMap<&'static str, PropValue> {
        BTreeMap::new()
    }
    /// Helper: empty BTreeSet<Arc<str>>
    fn empty_rgss_important() -> BTreeSet<Arc<str>> {
        BTreeSet::new()
    }
    /// Helper: empty BTreeSet<&'static str>
    fn empty_inline_important() -> BTreeSet<&'static str> {
        BTreeSet::new()
    }

    #[test]
    fn merge_inline_overrides_rgss() {
        let mut default = BTreeMap::new();
        default.insert("font-size", PropValue::str("14px"));

        let mut rgss = BTreeMap::new();
        rgss.insert(Arc::from("font-size"), PropValue::str("16px"));
        rgss.insert(Arc::from("color"), PropValue::str("black"));

        let mut inline = BTreeMap::new();
        inline.insert("font-size", PropValue::str("20px"));

        let theme = Theme::light();
        let result = StyleMerger::merge(
            &default,
            &rgss,
            &empty_rgss_important(),
            &inline,
            &empty_inline_important(),
            &theme,
            None,
        );

        assert_eq!(
            result.get("font-size").map(|v| format!("{v:?}")),
            Some(r#"Str("20px")"#.to_string())
        );
        assert!(result.contains_key("color"));
    }

    #[test]
    fn merge_var_resolution() {
        let mut default = BTreeMap::new();
        default.insert("background-color", PropValue::str("var(--color-primary)"));

        let theme = Theme::light();
        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        // var() 被解析为主题变量值
        let bg = result.get("background-color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        assert!(bg.is_some());
        assert_ne!(bg.unwrap(), "var(--color-primary)"); // 不应再是 var() 引用
    }

    // ========================================================================
    // !important 测试（ST07a RED）
    // ========================================================================

    /// rgss 中 `!important` 声明覆盖 inline 中的非 important 同名属性
    #[test]
    fn merge_important_rgss_overrides_inline_non_important() {
        let default = BTreeMap::new();

        // rgss: color: red !important
        let mut rgss = BTreeMap::new();
        rgss.insert(Arc::from("color"), PropValue::str("red"));
        let mut rgss_important = BTreeSet::new();
        rgss_important.insert(Arc::from("color"));

        // inline: color: blue（非 important）
        let mut inline = BTreeMap::new();
        inline.insert("color", PropValue::str("blue"));

        let theme = Theme::light();
        let result = StyleMerger::merge(
            &default,
            &rgss,
            &rgss_important,
            &inline,
            &empty_inline_important(),
            &theme,
            None,
        );

        // !important 的 red 应覆盖 inline 的 blue
        assert_eq!(
            result.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("red")"#.to_string())
        );
    }

    /// inline 中 `!important` 覆盖 rgss 的非 important 同名属性
    #[test]
    fn merge_important_inline_overrides_rgss_non_important() {
        let default = BTreeMap::new();

        // rgss: color: black（非 important）
        let mut rgss = BTreeMap::new();
        rgss.insert(Arc::from("color"), PropValue::str("black"));

        // inline: color: green !important
        let mut inline = BTreeMap::new();
        inline.insert("color", PropValue::str("green"));
        let mut inline_important: BTreeSet<&'static str> = BTreeSet::new();
        inline_important.insert("color");

        let theme = Theme::light();
        let result = StyleMerger::merge(
            &default,
            &rgss,
            &empty_rgss_important(),
            &inline,
            &inline_important,
            &theme,
            None,
        );

        assert_eq!(
            result.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("green")"#.to_string())
        );
    }

    /// 同一层同时有 important 和非 important 属性
    #[test]
    fn merge_important_mixed_with_normal() {
        let default = BTreeMap::new();

        // rgss: color: red !important, font-size: 14px
        let mut rgss = BTreeMap::new();
        rgss.insert(Arc::from("color"), PropValue::str("red"));
        rgss.insert(Arc::from("font-size"), PropValue::str("14px"));
        let mut rgss_important = BTreeSet::new();
        rgss_important.insert(Arc::from("color"));

        // inline: color: blue, font-size: 20px（覆盖非 important 的 font-size）
        let mut inline = BTreeMap::new();
        inline.insert("color", PropValue::str("blue"));
        inline.insert("font-size", PropValue::str("20px"));

        let theme = Theme::light();
        let result = StyleMerger::merge(
            &default,
            &rgss,
            &rgss_important,
            &inline,
            &empty_inline_important(),
            &theme,
            None,
        );

        // color: rgss !important 覆盖 inline 的 blue
        assert_eq!(
            result.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("red")"#.to_string())
        );
        // font-size: inline 正常覆盖 rgss
        assert_eq!(
            result.get("font-size").map(|v| format!("{v:?}")),
            Some(r#"Str("20px")"#.to_string())
        );
    }

    /// 高层 !important 覆盖低层 !important
    #[test]
    fn merge_important_higher_layer_overrides_lower() {
        let default = BTreeMap::new();

        // rgss: color: red !important
        let mut rgss = BTreeMap::new();
        rgss.insert(Arc::from("color"), PropValue::str("red"));
        let mut rgss_important = BTreeSet::new();
        rgss_important.insert(Arc::from("color"));

        // inline: color: blue !important
        let mut inline = BTreeMap::new();
        inline.insert("color", PropValue::str("blue"));
        let mut inline_important: BTreeSet<&'static str> = BTreeSet::new();
        inline_important.insert("color");

        let theme = Theme::light();
        let result = StyleMerger::merge(
            &default,
            &rgss,
            &rgss_important,
            &inline,
            &inline_important,
            &theme,
            None,
        );

        // 高层 !important 覆盖低层 !important
        assert_eq!(
            result.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("blue")"#.to_string())
        );
    }

    // ========================================================================
    // M06: CSS 变量循环引用检测测试（D4 §10）
    // ========================================================================

    /// 简单变量解析：var(--color-primary) → #3B82F6
    #[test]
    fn var_resolution_simple() {
        let mut default = BTreeMap::new();
        default.insert("background-color", PropValue::str("var(--color-primary)"));

        let theme = Theme::light();
        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        let bg = result.get("background-color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        assert_eq!(bg, Some("#3B82F6"));
    }

    /// 链式变量解析：--a → --b → 具体值
    #[test]
    fn var_resolution_chained() {
        let mut default = BTreeMap::new();
        default.insert("color", PropValue::str("var(--a)"));

        // 构建主题：--a: var(--b), --b: #ff0000
        let mut theme = Theme::new("test", ColorScheme::Light);
        theme.variables.insert("--a", PropValue::str("var(--b)"));
        theme.variables.insert("--b", PropValue::str("#ff0000"));

        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        let color = result.get("color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        assert_eq!(color, Some("#ff0000"));
    }

    /// 直接自循环：--x: var(--x) → 检测循环，保留原始引用
    #[test]
    fn var_cycle_direct_self_reference() {
        let mut default = BTreeMap::new();
        default.insert("color", PropValue::str("var(--x)"));

        let mut theme = Theme::new("test", ColorScheme::Light);
        theme.variables.insert("--x", PropValue::str("var(--x)"));

        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        // 循环检测：不应 panic，且 result 中 color 保持原始 var(--x) 引用（降级）
        let color = result.get("color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        // 循环时保留原始引用作为降级
        assert_eq!(color, Some("var(--x)"));
    }

    /// 间接循环：--a → --b → --a
    #[test]
    fn var_cycle_two_var_loop() {
        let mut default = BTreeMap::new();
        default.insert("color", PropValue::str("var(--a)"));

        let mut theme = Theme::new("test", ColorScheme::Light);
        theme.variables.insert("--a", PropValue::str("var(--b)"));
        theme.variables.insert("--b", PropValue::str("var(--a)"));

        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        let color = result.get("color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        // 循环检测：a 已访问，遇到 b→var(--a) 时检测到循环
        assert_eq!(color, Some("var(--a)"));
    }

    /// 三变量循环：--a → --b → --c → --a
    #[test]
    fn var_cycle_three_var_loop() {
        let mut default = BTreeMap::new();
        default.insert("color", PropValue::str("var(--a)"));

        let mut theme = Theme::new("test", ColorScheme::Light);
        theme.variables.insert("--a", PropValue::str("var(--b)"));
        theme.variables.insert("--b", PropValue::str("var(--c)"));
        theme.variables.insert("--c", PropValue::str("var(--a)"));

        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        let color = result.get("color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        // 循环检测：a→b→c→a 检测到循环
        assert_eq!(color, Some("var(--a)"));
    }

    /// 深度链式解析（无循环）：4 层
    #[test]
    fn var_resolution_deep_chain() {
        let mut default = BTreeMap::new();
        default.insert("color", PropValue::str("var(--a)"));

        let mut theme = Theme::new("test", ColorScheme::Light);
        theme.variables.insert("--a", PropValue::str("var(--b)"));
        theme.variables.insert("--b", PropValue::str("var(--c)"));
        theme.variables.insert("--c", PropValue::str("var(--d)"));
        theme.variables.insert("--d", PropValue::str("#deep-value"));

        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        let color = result.get("color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        assert_eq!(color, Some("#deep-value"));
    }

    /// 缺失变量：var(--nonexistent) → 保留原始引用（不崩溃）
    #[test]
    fn var_missing_variable_keeps_reference() {
        let mut default = BTreeMap::new();
        default.insert("color", PropValue::str("var(--nonexistent)"));

        let theme = Theme::new("test", ColorScheme::Light);
        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        let color = result.get("color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        // 未定义变量：保留原始 var() 引用
        assert_eq!(color, Some("var(--nonexistent)"));
    }

    /// 部分链解析中断：--a → --b，--b 不存在 → 保留 --a 的引用
    #[test]
    fn var_broken_chain_keeps_reference() {
        let mut default = BTreeMap::new();
        default.insert("color", PropValue::str("var(--a)"));

        let mut theme = Theme::new("test", ColorScheme::Light);
        theme.variables.insert("--a", PropValue::str("var(--b)"));
        // --b 未定义

        let result = StyleMerger::merge(
            &default,
            &empty_rgss_map(),
            &empty_rgss_important(),
            &empty_inline_map(),
            &empty_inline_important(),
            &theme,
            None,
        );

        let color = result.get("color").and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(s.as_ref())
            } else {
                None
            }
        });
        // 链中断：保留原始引用
        assert_eq!(color, Some("var(--a)"));
    }
}
