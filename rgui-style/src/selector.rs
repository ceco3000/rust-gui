//! 选择器引擎——Selector、Specificity、匹配算法。
//!
//! 定义源自 D4 §4。

use rgui_core::view::PropValue;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

// ============================================================================
// Specificity
// ============================================================================

/// CSS 特异性三元组 (a, b, c)，对应 ID/Class/Type（D4 §4.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Specificity {
    /// ID 选择器贡献。
    pub const ID: Self = Self(1, 0, 0);
    /// Class/属性/伪类选择器贡献。
    pub const CLASS: Self = Self(0, 1, 0);
    /// 类型选择器贡献。
    pub const TYPE: Self = Self(0, 0, 1);
    /// 零特异性（最低）。
    pub const ZERO: Self = Self(0, 0, 0);
}

// ============================================================================
// CombinatorKind
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombinatorKind { Descendant, Child }

// ============================================================================
// Selector
// ============================================================================

/// CSS 选择器（D4 §4.1）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Selector {
    /// 类型选择器：`Button`
    Type(String),
    /// 类选择器：`.page`
    Class(String),
    /// ID 选择器：`#main-header`
    Id(String),
    /// 属性选择器：`[variant="primary"]`
    Attribute { name: String, value: Option<String> },
    /// 伪类选择器：`:hover`
    PseudoClass(String),
    /// 组合器：`ancestor > descendant`
    Combinator { ancestor: Box<Selector>, descendant: Box<Selector>, kind: CombinatorKind },
}

impl Selector {
    /// 计算选择器的特异性。
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        match self {
            Self::Id(_) => Specificity::ID,
            Self::Class(_) | Self::Attribute { .. } | Self::PseudoClass(_) => Specificity::CLASS,
            Self::Type(_) => Specificity::TYPE,
            Self::Combinator { ancestor, descendant, .. } => {
                let a = ancestor.specificity();
                let d = descendant.specificity();
                Specificity(a.0 + d.0, a.1 + d.1, a.2 + d.2)
            }
        }
    }

    /// 检查选择器是否匹配 widget（D4 §4.2）。
    ///
    /// 当前实现：仅匹配叶子选择器（类型/类/ID/属性/伪类）。
    /// 组合器匹配将在完整实现中支持。
    #[must_use]
    pub fn matches(
        &self,
        widget_type: &str,
        class_list: &[&str],
        _pseudo_states: &[&str],
        attr_map: &BTreeMap<&str, PropValue>,
    ) -> bool {
        match self {
            Self::Type(t) => {
                // 匹配 widget_type 的短名
                widget_type == t.as_str() || widget_type.ends_with(&format!("::{t}"))
            }
            Self::Class(c) => class_list.contains(&c.as_str()),
            Self::Id(_id) => {
                // ID 匹配需要 widget 的 stable_id，当前简化为检查属性
                attr_map.get("id").is_some_and(|v| matches!(v, PropValue::Str(s) if s.as_ref() == _id.as_str()))
            }
            Self::Attribute { name, value } => {
                match attr_map.get(name.as_str()) {
                    Some(attr_val) => match value {
                        Some(expected) => {
                            // 简单字符串比较
                            format!("{attr_val:?}").contains(expected.as_str())
                        }
                        None => true, // [disabled] 等布尔属性——存在即匹配
                    },
                    None => false,
                }
            }
            Self::PseudoClass(_pc) => {
                // 伪类匹配需要运行时状态（焦点、悬停等），当前简化返回 true
                true
            }
            Self::Combinator { descendant, .. } => {
                // 简化：仅检查后代选择器
                descendant.matches(widget_type, class_list, _pseudo_states, attr_map)
            }
        }
    }

    /// 创建类型选择器。
    #[must_use]
    pub fn type_selector(name: impl Into<String>) -> Self {
        Self::Type(name.into())
    }

    /// 创建类选择器。
    #[must_use]
    pub fn class(name: impl Into<String>) -> Self {
        Self::Class(name.into())
    }

    /// 创建后代组合器。
    #[must_use]
    pub fn descendant(ancestor: Selector, descendant: Selector) -> Self {
        Self::Combinator {
            ancestor: Box::new(ancestor),
            descendant: Box::new(descendant),
            kind: CombinatorKind::Descendant,
        }
    }
}

// ============================================================================
// StyleRule
// ============================================================================

/// 样式规则（D4 §4.2）。
#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: Selector,
    /// 属性名 → 属性值
    pub declarations: BTreeMap<Arc<str>, PropValue>,
    pub specificity: Specificity,
}

impl StyleRule {
    #[must_use]
    pub fn new(selector: Selector, declarations: BTreeMap<Arc<str>, PropValue>) -> Self {
        let specificity = selector.specificity();
        Self { selector, declarations, specificity }
    }
}

// ============================================================================
// SelectorEngine
// ============================================================================

/// 选择器引擎（D4 §4.2）。
///
/// 存储已解析的样式规则并提供匹配能力。
#[derive(Default)]
pub struct SelectorEngine {
    rules: Vec<StyleRule>,
}

impl SelectorEngine {
    #[must_use]
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 注册一条样式规则。
    pub fn add_rule(&mut self, rule: StyleRule) {
        self.rules.push(rule);
    }

    /// 匹配 widget 的所有适用规则，按 specificity 升序合并。
    ///
    /// 后应用的规则覆盖先应用的规则（高 specificity 覆盖低 specificity）。
    #[must_use]
    pub fn match_widget(
        &self,
        widget_type: &str,
        class_list: &[&str],
        pseudo_states: &[&str],
        attr_map: &BTreeMap<&str, PropValue>,
    ) -> BTreeMap<Arc<str>, PropValue> {
        let mut matched: Vec<&StyleRule> = self
            .rules
            .iter()
            .filter(|r| {
                r.selector.matches(widget_type, class_list, pseudo_states, attr_map)
            })
            .collect();

        matched.sort_by_key(|r| r.specificity);

        let mut result = BTreeMap::new();
        for rule in &matched {
            for (prop, value) in &rule.declarations {
                result.insert(Arc::clone(prop), value.clone());
            }
        }
        result
    }

    /// 已注册的规则数量。
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl fmt::Debug for SelectorEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorEngine")
            .field("rules", &self.rules.len())
            .finish()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specificity_ordering() {
        assert!(Specificity::ID > Specificity::CLASS);
        assert!(Specificity::CLASS > Specificity::TYPE);
        assert!(Specificity::TYPE > Specificity::ZERO);
    }

    #[test]
    fn selector_type_matches() {
        let s = Selector::type_selector("Button");
        assert!(s.matches("Button", &[], &[], &BTreeMap::new()));
    }

    #[test]
    fn selector_type_not_matches_different() {
        let s = Selector::type_selector("Button");
        assert!(!s.matches("TextField", &[], &[], &BTreeMap::new()));
    }

    #[test]
    fn selector_class_matches() {
        let s = Selector::class("primary");
        assert!(s.matches("Button", &["primary"], &[], &BTreeMap::new()));
    }

    #[test]
    fn selector_class_not_matches() {
        let s = Selector::class("primary");
        assert!(!s.matches("Button", &["secondary"], &[], &BTreeMap::new()));
    }

    #[test]
    fn selector_attribute_exists() {
        let s = Selector::Attribute {
            name: "disabled".into(),
            value: None,
        };
        let mut attrs = BTreeMap::new();
        attrs.insert("disabled", PropValue::Bool(true));
        assert!(s.matches("Button", &[], &[], &attrs));
    }

    #[test]
    fn engine_match_and_merge() {
        let mut engine = SelectorEngine::new();

        // 低特异性规则
        engine.add_rule(StyleRule::new(
            Selector::type_selector("Button"),
            {
                let mut m = BTreeMap::new();
                m.insert(Arc::from("font-size"), PropValue::str("14px"));
                m
            },
        ));

        // 高特异性规则
        engine.add_rule(StyleRule::new(
            Selector::class("primary"),
            {
                let mut m = BTreeMap::new();
                m.insert(Arc::from("color"), PropValue::str("blue"));
                m.insert(Arc::from("font-size"), PropValue::str("16px"));
                m
            },
        ));

        let result = engine.match_widget("Button", &["primary"], &[], &BTreeMap::new());
        // 高特异性覆盖 font-size
        assert_eq!(result.get("font-size").map(|v| format!("{v:?}")),
                   Some(r#"Str("16px")"#.to_string()));
        assert!(result.contains_key("color"));
    }

    #[test]
    fn selector_specificity_sum() {
        let s = Selector::descendant(
            Selector::type_selector("VBox"),
            Selector::class("primary"),
        );
        assert_eq!(s.specificity(), Specificity(0, 1, 1));
    }
}
