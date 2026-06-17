//! 样式合并——4 层优先级合并。
//!
//! 定义源自 D4 §6。

use crate::theme::Theme;
use rgui_core::view::PropValue;
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
    #[must_use]
    pub fn merge(
        default_style: &BTreeMap<&'static str, PropValue>,
        rgss_matched: &BTreeMap<Arc<str>, PropValue>,
        rgss_important: &BTreeSet<Arc<str>>,
        inline_style: &BTreeMap<&'static str, PropValue>,
        inline_important: &BTreeSet<&'static str>,
        theme: &Theme,
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
        let var_prefix = "var(";
        let keys: Vec<Arc<str>> = result.keys().cloned().collect();
        for key in keys {
            if let Some(PropValue::Str(ref s)) = result.get(&key) {
                if s.starts_with(var_prefix) {
                    let var_name = s
                        .trim_start_matches(var_prefix)
                        .trim_end_matches(')')
                        .trim();
                    if let Some(resolved) = theme.var(var_name) {
                        result.insert(key, resolved.clone());
                    }
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
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

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
        );

        // 高层 !important 覆盖低层 !important
        assert_eq!(
            result.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("blue")"#.to_string())
        );
    }
}
