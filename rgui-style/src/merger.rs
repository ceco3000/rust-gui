//! 样式合并——4 层优先级合并。
//!
//! 定义源自 D4 §6。

use crate::theme::Theme;
use rgui_core::view::PropValue;
use std::collections::BTreeMap;
use std::sync::Arc;

/// 样式合并器（D4 §6.2）。
///
/// 按优先级从低到高合并样式属性：
/// 1. 框架默认样式
/// 2. .rgss 匹配的规则
/// 3. 组件 inline style
/// 4. 主题变量解析
pub struct StyleMerger;

impl StyleMerger {
    /// 合并四层样式，返回最终属性集。
    ///
    /// 后一层覆盖前一层的同名属性。
    #[must_use]
    pub fn merge(
        default_style: &BTreeMap<&'static str, PropValue>,
        rgss_matched: &BTreeMap<Arc<str>, PropValue>,
        inline_style: &BTreeMap<&'static str, PropValue>,
        theme: &Theme,
    ) -> BTreeMap<Arc<str>, PropValue> {
        let mut result = BTreeMap::new();

        // 第 1 层：框架默认样式
        for (&key, value) in default_style {
            result.insert(Arc::from(key), value.clone());
        }

        // 第 2 层：.rgss 匹配
        for (key, value) in rgss_matched {
            result.insert(Arc::clone(key), value.clone());
        }

        // 第 3 层：inline style
        for (&key, value) in inline_style {
            result.insert(Arc::from(key), value.clone());
        }

        // 第 4 层：解析 var() 引用（简化：检查值是否以 "var(" 开头）
        let var_prefix = "var(";
        let keys: Vec<Arc<str>> = result.keys().cloned().collect();
        for key in keys {
            if let Some(PropValue::Str(ref s)) = result.get(&key) {
                if s.starts_with(var_prefix) {
                    let var_name = s
                        .trim_start_matches("var(")
                        .trim_end_matches(')')
                        .trim();
                    if let Some(resolved) = theme.var(var_name) {
                        result.insert(key, resolved.clone());
                    }
                }
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
        let result = StyleMerger::merge(&default, &rgss, &inline, &theme);

        assert_eq!(result.get("font-size").map(|v| format!("{v:?}")),
                   Some(r#"Str("20px")"#.to_string()));
        assert!(result.contains_key("color"));
    }

    #[test]
    fn merge_var_resolution() {
        let mut default = BTreeMap::new();
        default.insert("background-color", PropValue::str("var(--color-primary)"));

        let theme = Theme::light();
        let result = StyleMerger::merge(&default, &BTreeMap::new(), &BTreeMap::new(), &theme);

        // var() 被解析为主题变量值
        let bg = result.get("background-color").and_then(|v| {
            if let PropValue::Str(s) = v { Some(s.as_ref()) } else { None }
        });
        assert!(bg.is_some());
        assert_ne!(bg.unwrap(), "var(--color-primary)"); // 不应再是 var() 引用
    }
}
