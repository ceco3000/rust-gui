//! 选择器引擎——Selector、Specificity、匹配算法、媒体查询。
//!
//! 定义源自 D4 §4、§8。

use rgui_core::view::PropValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

// ============================================================================
// 断点常量（D4 §8）
// ============================================================================

/// 最小断点宽度常量表（D4 §8）。
pub mod breakpoints {
    /// `xs`：手机竖屏（0px）。
    pub const XS: f64 = 0.0;
    /// `sm`：手机横屏（640px）。
    pub const SM: f64 = 640.0;
    /// `md`：平板（768px）。
    pub const MD: f64 = 768.0;
    /// `lg`：笔记本（1024px）。
    pub const LG: f64 = 1024.0;
    /// `xl`：桌面显示器（1280px）。
    pub const XL: f64 = 1280.0;
    /// `2xl`：大屏（1536px）。
    pub const XXL: f64 = 1536.0;
}

// ============================================================================
// MediaCondition
// ============================================================================

/// 媒体查询条件（D4 §8）。
///
/// 表示 `@media` 规则中的条件表达式，支持：
/// - `max-width` 上限
/// - `min-width` 下限
/// - `prefers-color-scheme` 色彩方案偏好
/// - `and` 复合条件
#[derive(Debug, Clone, PartialEq)]
pub enum MediaCondition {
    /// `max-width: <px>`：窗口宽度小于等于指定值（含等号边界）。
    MaxWidth(f64),
    /// `min-width: <px>`：窗口宽度大于等于指定值（含等号边界）。
    MinWidth(f64),
    /// `prefers-color-scheme: light/dark`：用户色彩方案偏好。
    PrefersColorScheme(crate::theme::ColorScheme),
    /// `and` 复合条件：所有子条件同时满足时成立。
    And(Vec<MediaCondition>),
}

impl MediaCondition {
    /// 评估媒体查询条件是否在当前环境下成立。
    ///
    /// # 参数
    ///
    /// * `window_width` — 当前窗口逻辑宽度（像素）。
    /// * `color_scheme` — 当前色彩方案。
    #[must_use]
    pub fn eval(&self, window_width: f64, color_scheme: crate::theme::ColorScheme) -> bool {
        match self {
            Self::MaxWidth(threshold) => window_width <= *threshold,
            Self::MinWidth(threshold) => window_width >= *threshold,
            Self::PrefersColorScheme(scheme) => color_scheme == *scheme,
            Self::And(conditions) => conditions
                .iter()
                .all(|c| c.eval(window_width, color_scheme)),
        }
    }
}

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
pub enum CombinatorKind {
    Descendant,
    Child,
}

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
    /// 复合类选择器：`.primary.large`——匹配同时拥有所有指定类的元素
    Classes(Vec<String>),
    /// ID 选择器：`#main-header`
    Id(String),
    /// 属性选择器：`[variant="primary"]`
    Attribute { name: String, value: Option<String> },
    /// 伪类选择器：`:hover`
    PseudoClass(String),
    /// 组合器：`ancestor > descendant`
    Combinator {
        ancestor: Box<Selector>,
        descendant: Box<Selector>,
        kind: CombinatorKind,
    },
}

impl Selector {
    /// 计算选择器的特异性。
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        match self {
            Self::Id(_) => Specificity::ID,
            Self::Class(_) | Self::Attribute { .. } | Self::PseudoClass(_) => Specificity::CLASS,
            Self::Classes(classes) => {
                if classes.is_empty() {
                    Specificity::ZERO
                } else {
                    Specificity(0, classes.len() as u32, 0)
                }
            },
            Self::Type(_) => Specificity::TYPE,
            Self::Combinator {
                ancestor,
                descendant,
                ..
            } => {
                let a = ancestor.specificity();
                let d = descendant.specificity();
                Specificity(a.0 + d.0, a.1 + d.1, a.2 + d.2)
            },
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
            },
            Self::Class(c) => class_list.contains(&c.as_str()),
            Self::Classes(classes) => {
                // 复合类选择器：所有类都必须存在于 class_list 中
                if classes.is_empty() {
                    return false;
                }
                classes.iter().all(|c| class_list.contains(&c.as_str()))
            },
            Self::Id(_id) => {
                // ID 匹配需要 widget 的 stable_id，当前简化为检查属性
                attr_map
                    .get("id")
                    .is_some_and(|v| matches!(v, PropValue::Str(s) if s.as_ref() == _id.as_str()))
            },
            Self::Attribute { name, value } => {
                match attr_map.get(name.as_str()) {
                    Some(attr_val) => match value {
                        Some(expected) => {
                            // 简单字符串比较
                            format!("{attr_val:?}").contains(expected.as_str())
                        },
                        None => true, // [disabled] 等布尔属性——存在即匹配
                    },
                    None => false,
                }
            },
            Self::PseudoClass(_pc) => {
                // 伪类匹配需要运行时状态（焦点、悬停等），当前简化返回 true
                true
            },
            Self::Combinator { descendant, .. } => {
                // 简化：仅检查后代选择器
                descendant.matches(widget_type, class_list, _pseudo_states, attr_map)
            },
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

    /// 创建复合类选择器（如 `.primary.large`）。
    ///
    /// 匹配时要求元素拥有 ALL 指定的 CSS 类。
    /// 空列表的复合选择器不匹配任何元素。
    #[must_use]
    pub fn classes(names: &[&str]) -> Self {
        Self::Classes(names.iter().map(|s| s.to_string()).collect())
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
///
/// 每条规则由选择器、声明块和可选媒体查询条件组成。
/// 媒体查询条件在匹配时按当前窗口尺寸和色彩方案过滤。
#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: Selector,
    /// 属性名 → 属性值
    pub declarations: BTreeMap<Arc<str>, PropValue>,
    pub specificity: Specificity,
    /// 可选的媒体查询条件（D4 §8）。
    /// 非 `None` 时，仅当条件满足时规则才生效。
    pub media_condition: Option<MediaCondition>,
    /// 标记了 `!important` 的属性名集合（D4 §6.1 第 5 优先级）。
    pub important_declarations: BTreeSet<Arc<str>>,
}

impl StyleRule {
    #[must_use]
    pub fn new(selector: Selector, declarations: BTreeMap<Arc<str>, PropValue>) -> Self {
        Self::with_important(selector, declarations, BTreeSet::new())
    }

    /// 创建带 `!important` 标记的样式规则。
    #[must_use]
    pub fn with_important(
        selector: Selector,
        declarations: BTreeMap<Arc<str>, PropValue>,
        important_declarations: BTreeSet<Arc<str>>,
    ) -> Self {
        let specificity = selector.specificity();
        Self {
            selector,
            declarations,
            specificity,
            media_condition: None,
            important_declarations,
        }
    }

    /// 创建带媒体查询条件和 `!important` 标记的样式规则。
    #[must_use]
    pub fn with_media_and_important(
        selector: Selector,
        declarations: BTreeMap<Arc<str>, PropValue>,
        media_condition: MediaCondition,
        important_declarations: BTreeSet<Arc<str>>,
    ) -> Self {
        let specificity = selector.specificity();
        Self {
            selector,
            declarations,
            specificity,
            media_condition: Some(media_condition),
            important_declarations,
        }
    }

    /// 创建带媒体查询条件的样式规则（无 `!important`）。
    #[must_use]
    pub fn with_media(
        selector: Selector,
        declarations: BTreeMap<Arc<str>, PropValue>,
        media_condition: MediaCondition,
    ) -> Self {
        Self::with_media_and_important(selector, declarations, media_condition, BTreeSet::new())
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
    /// 此方法忽略媒体查询条件（等价于所有媒体条件均视为满足）。
    #[must_use]
    pub fn match_widget(
        &self,
        widget_type: &str,
        class_list: &[&str],
        pseudo_states: &[&str],
        attr_map: &BTreeMap<&str, PropValue>,
    ) -> BTreeMap<Arc<str>, PropValue> {
        self.match_widget_with_media(
            widget_type,
            class_list,
            pseudo_states,
            attr_map,
            0.0,
            crate::theme::ColorScheme::Light,
        )
    }

    /// 匹配 widget 的所有适用规则，支持媒体查询条件过滤（D4 §8）。
    ///
    /// # 参数
    ///
    /// * `widget_type` — widget 类型名（如 `"Button"`）
    /// * `class_list` — widget 的 CSS 类列表
    /// * `pseudo_states` — widget 的伪类状态列表
    /// * `attr_map` — widget 的属性映射
    /// * `window_width` — 当前窗口逻辑宽度（px），用于评估 `max-width`/`min-width` 条件
    /// * `color_scheme` — 当前色彩方案
    #[must_use]
    pub fn match_widget_with_media(
        &self,
        widget_type: &str,
        class_list: &[&str],
        pseudo_states: &[&str],
        attr_map: &BTreeMap<&str, PropValue>,
        window_width: f64,
        color_scheme: crate::theme::ColorScheme,
    ) -> BTreeMap<Arc<str>, PropValue> {
        let mut matched: Vec<&StyleRule> = self
            .rules
            .iter()
            .filter(|r| {
                // 先检查选择器是否匹配
                if !r
                    .selector
                    .matches(widget_type, class_list, pseudo_states, attr_map)
                {
                    return false;
                }
                // 再检查媒体查询条件（有条件时需通过 eval）
                match &r.media_condition {
                    Some(condition) => condition.eval(window_width, color_scheme),
                    None => true,
                }
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
    use crate::ColorScheme;

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
        engine.add_rule(StyleRule::new(Selector::type_selector("Button"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("font-size"), PropValue::str("14px"));
            m
        }));

        // 高特异性规则
        engine.add_rule(StyleRule::new(Selector::class("primary"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("color"), PropValue::str("blue"));
            m.insert(Arc::from("font-size"), PropValue::str("16px"));
            m
        }));

        let result = engine.match_widget("Button", &["primary"], &[], &BTreeMap::new());
        // 高特异性覆盖 font-size
        assert_eq!(
            result.get("font-size").map(|v| format!("{v:?}")),
            Some(r#"Str("16px")"#.to_string())
        );
        assert!(result.contains_key("color"));
    }

    #[test]
    fn selector_specificity_sum() {
        let s = Selector::descendant(Selector::type_selector("VBox"), Selector::class("primary"));
        assert_eq!(s.specificity(), Specificity(0, 1, 1));
    }

    // ========================================================================
    // MediaCondition 测试
    // ========================================================================

    #[test]
    fn media_max_width_pass() {
        let cond = MediaCondition::MaxWidth(768.0);
        assert!(cond.eval(768.0, ColorScheme::Light)); // 等于边界
        assert!(cond.eval(600.0, ColorScheme::Light)); // 小于
        assert!(!cond.eval(769.0, ColorScheme::Light)); // 大于
    }

    #[test]
    fn media_min_width_pass() {
        let cond = MediaCondition::MinWidth(768.0);
        assert!(cond.eval(768.0, ColorScheme::Light)); // 等于边界
        assert!(cond.eval(1024.0, ColorScheme::Light)); // 大于
        assert!(!cond.eval(600.0, ColorScheme::Light)); // 小于
    }

    #[test]
    fn media_prefers_color_scheme() {
        let dark = MediaCondition::PrefersColorScheme(ColorScheme::Dark);
        assert!(dark.eval(800.0, ColorScheme::Dark));
        assert!(!dark.eval(800.0, ColorScheme::Light));

        let light = MediaCondition::PrefersColorScheme(ColorScheme::Light);
        assert!(light.eval(800.0, ColorScheme::Light));
        assert!(!light.eval(800.0, ColorScheme::Dark));
    }

    #[test]
    fn media_and_composite_both_pass() {
        let cond = MediaCondition::And(vec![
            MediaCondition::MinWidth(640.0),
            MediaCondition::MaxWidth(1024.0),
        ]);
        assert!(cond.eval(768.0, ColorScheme::Light)); // 在范围内
        assert!(cond.eval(640.0, ColorScheme::Light)); // 下边界
        assert!(cond.eval(1024.0, ColorScheme::Light)); // 上边界
    }

    #[test]
    fn media_and_composite_one_fails() {
        let cond = MediaCondition::And(vec![
            MediaCondition::MinWidth(640.0),
            MediaCondition::PrefersColorScheme(ColorScheme::Dark),
        ]);
        // 宽度满足，色彩方案不满足
        assert!(!cond.eval(800.0, ColorScheme::Light));
        // 两者都满足
        assert!(cond.eval(800.0, ColorScheme::Dark));
    }

    #[test]
    fn media_and_composite_empty_is_true() {
        let cond = MediaCondition::And(vec![]);
        assert!(cond.eval(0.0, ColorScheme::Light));
    }

    #[test]
    fn media_max_width_zero_boundary() {
        let cond = MediaCondition::MaxWidth(0.0);
        assert!(cond.eval(0.0, ColorScheme::Light));
        assert!(!cond.eval(0.1, ColorScheme::Light));
    }

    #[test]
    fn media_min_width_zero_boundary() {
        let cond = MediaCondition::MinWidth(0.0);
        assert!(cond.eval(0.0, ColorScheme::Light));
        assert!(cond.eval(0.1, ColorScheme::Light));
    }

    #[test]
    fn media_decimal_threshold() {
        let cond = MediaCondition::MaxWidth(768.5);
        assert!(cond.eval(768.5, ColorScheme::Light));
        assert!(cond.eval(768.0, ColorScheme::Light));
        assert!(!cond.eval(769.0, ColorScheme::Light));
    }

    // ========================================================================
    // match_widget_with_media 测试
    // ========================================================================

    #[test]
    fn match_widget_with_media_max_width_activated() {
        let mut engine = SelectorEngine::new();

        // 无媒体条件的规则（始终生效）
        engine.add_rule(StyleRule::new(Selector::type_selector("Button"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("color"), PropValue::str("red"));
            m
        }));

        // 带媒体查询的规则：max-width: 768px，小屏时应覆盖
        engine.add_rule(StyleRule::with_media(
            Selector::type_selector("Button"),
            {
                let mut m = BTreeMap::new();
                m.insert(Arc::from("color"), PropValue::str("blue"));
                m
            },
            MediaCondition::MaxWidth(768.0),
        ));

        // 窗口宽度 600 < 768，媒体条件满足 → color=blue
        let narrow = engine.match_widget_with_media(
            "Button",
            &[],
            &[],
            &BTreeMap::new(),
            600.0,
            ColorScheme::Light,
        );
        assert_eq!(
            narrow.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("blue")"#.to_string()),
        );

        // 窗口宽度 1024 > 768，媒体条件不满足 → color=red
        let wide = engine.match_widget_with_media(
            "Button",
            &[],
            &[],
            &BTreeMap::new(),
            1024.0,
            ColorScheme::Light,
        );
        assert_eq!(
            wide.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("red")"#.to_string()),
        );
    }

    #[test]
    fn match_widget_with_media_prefers_color_scheme() {
        let mut engine = SelectorEngine::new();

        engine.add_rule(StyleRule::new(Selector::type_selector("Label"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("color"), PropValue::str("black"));
            m
        }));

        // 暗色主题覆盖
        engine.add_rule(StyleRule::with_media(
            Selector::type_selector("Label"),
            {
                let mut m = BTreeMap::new();
                m.insert(Arc::from("color"), PropValue::str("white"));
                m
            },
            MediaCondition::PrefersColorScheme(ColorScheme::Dark),
        ));

        // 亮色 → black
        let light = engine.match_widget_with_media(
            "Label",
            &[],
            &[],
            &BTreeMap::new(),
            800.0,
            ColorScheme::Light,
        );
        assert_eq!(
            light.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("black")"#.to_string()),
        );

        // 暗色 → white
        let dark = engine.match_widget_with_media(
            "Label",
            &[],
            &[],
            &BTreeMap::new(),
            800.0,
            ColorScheme::Dark,
        );
        assert_eq!(
            dark.get("color").map(|v| format!("{v:?}")),
            Some(r#"Str("white")"#.to_string()),
        );
    }

    #[test]
    fn match_widget_with_media_composite_and() {
        let mut engine = SelectorEngine::new();

        engine.add_rule(StyleRule::new(Selector::type_selector("VBox"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("padding"), PropValue::str("24px"));
            m
        }));

        // (min-width: 640px) and (max-width: 768px) 时覆盖 padding
        engine.add_rule(StyleRule::with_media(
            Selector::type_selector("VBox"),
            {
                let mut m = BTreeMap::new();
                m.insert(Arc::from("padding"), PropValue::str("12px"));
                m
            },
            MediaCondition::And(vec![
                MediaCondition::MinWidth(640.0),
                MediaCondition::MaxWidth(768.0),
            ]),
        ));

        // 窗口 500px，不满足 min-width → 24px
        let small = engine.match_widget_with_media(
            "VBox",
            &[],
            &[],
            &BTreeMap::new(),
            500.0,
            ColorScheme::Light,
        );
        assert_eq!(
            small.get("padding").map(|v| format!("{v:?}")),
            Some(r#"Str("24px")"#.to_string()),
        );

        // 窗口 700px，满足复合条件 → 12px
        let medium = engine.match_widget_with_media(
            "VBox",
            &[],
            &[],
            &BTreeMap::new(),
            700.0,
            ColorScheme::Light,
        );
        assert_eq!(
            medium.get("padding").map(|v| format!("{v:?}")),
            Some(r#"Str("12px")"#.to_string()),
        );

        // 窗口 1024px，不满足 max-width → 24px
        let large = engine.match_widget_with_media(
            "VBox",
            &[],
            &[],
            &BTreeMap::new(),
            1024.0,
            ColorScheme::Light,
        );
        assert_eq!(
            large.get("padding").map(|v| format!("{v:?}")),
            Some(r#"Str("24px")"#.to_string()),
        );
    }

    #[test]
    fn match_widget_with_media_no_media_rules_always_apply() {
        let mut engine = SelectorEngine::new();

        engine.add_rule(StyleRule::new(Selector::type_selector("Button"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("font-size"), PropValue::str("14px"));
            m
        }));

        // 不带媒体条件的规则在所有窗口尺寸下都生效
        for width in &[0.0, 640.0, 768.0, 1024.0, 1920.0] {
            let result = engine.match_widget_with_media(
                "Button",
                &[],
                &[],
                &BTreeMap::new(),
                *width,
                ColorScheme::Light,
            );
            assert_eq!(
                result.get("font-size").map(|v| format!("{v:?}")),
                Some(r#"Str("14px")"#.to_string()),
                "width={} 时无媒体规则应始终生效",
                width
            );
        }
    }

    #[test]
    fn breakpoint_constants() {
        assert_eq!(breakpoints::XS, 0.0);
        assert_eq!(breakpoints::SM, 640.0);
        assert_eq!(breakpoints::MD, 768.0);
        assert_eq!(breakpoints::LG, 1024.0);
        assert_eq!(breakpoints::XL, 1280.0);
        assert_eq!(breakpoints::XXL, 1536.0);
    }

    // ========================================================================
    // H03: class 属性处理——复合类选择器 + class_list 提取
    // ========================================================================

    /// 验收标准：`class="primary large"` → `.primary` 匹配
    #[test]
    fn h03_single_class_primary_matches() {
        let s = Selector::class("primary");
        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();
        assert!(s.matches("Button", &class_refs, &[], &BTreeMap::new()));
    }

    /// 验收标准：`class="primary large"` → `.large` 匹配
    #[test]
    fn h03_single_class_large_matches() {
        let s = Selector::class("large");
        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();
        assert!(s.matches("Button", &class_refs, &[], &BTreeMap::new()));
    }

    /// 验收标准：`class="primary large"` → 复合选择器 `.primary.large` 匹配（全部类均存在）
    #[test]
    fn h03_compound_classes_matches() {
        let s = Selector::classes(&["primary", "large"]);
        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();
        assert!(s.matches("Button", &class_refs, &[], &BTreeMap::new()));
    }

    /// 复合选择器：部分类不匹配 → 不匹配
    #[test]
    fn h03_compound_classes_partial_no_match() {
        let s = Selector::classes(&["primary", "other"]);
        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();
        assert!(!s.matches("Button", &class_refs, &[], &BTreeMap::new()));
    }

    /// 复合选择器特异性 = 每个类 CLASS 特异性之和
    #[test]
    fn h03_compound_classes_specificity() {
        // 两个类 → (0, 2, 0)
        let s = Selector::classes(&["primary", "large"]);
        assert_eq!(s.specificity(), Specificity(0, 2, 0));

        // 三个类 → (0, 3, 0)
        let s3 = Selector::classes(&["a", "b", "c"]);
        assert_eq!(s3.specificity(), Specificity(0, 3, 0));

        // 单个类 → (0, 1, 0)，与 Selector::Class 行为一致
        let s1 = Selector::classes(&["primary"]);
        assert_eq!(s1.specificity(), Specificity::CLASS);
    }

    /// 单类复合选择器行为等价于普通类选择器
    #[test]
    fn h03_compound_single_class_equiv() {
        let s_compound = Selector::classes(&["primary"]);
        let s_simple = Selector::class("primary");
        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();

        assert_eq!(s_compound.specificity(), s_simple.specificity());
        assert_eq!(
            s_compound.matches("Button", &class_refs, &[], &BTreeMap::new()),
            s_simple.matches("Button", &class_refs, &[], &BTreeMap::new())
        );
    }

    /// 空复合类选择器：解析不应产生空 Vec，但若产生则视为不匹配任何元素
    #[test]
    fn h03_empty_compound_classes_no_match() {
        let s = Selector::classes(&[]);
        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();
        assert!(!s.matches("Button", &class_refs, &[], &BTreeMap::new()));
    }

    /// 引擎级测试：类选择器通过引擎匹配
    #[test]
    fn h03_engine_class_matching() {
        let mut engine = SelectorEngine::new();

        engine.add_rule(StyleRule::new(Selector::class("primary"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("color"), PropValue::str("blue"));
            m
        }));

        engine.add_rule(StyleRule::new(Selector::class("large"), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("font-size"), PropValue::str("24px"));
            m
        }));

        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();
        let result = engine.match_widget("Button", &class_refs, &[], &BTreeMap::new());

        assert!(result.contains_key("color"));
        assert!(result.contains_key("font-size"));
    }

    /// 引擎级测试：复合类选择器通过引擎匹配
    #[test]
    fn h03_engine_compound_class_matching() {
        let mut engine = SelectorEngine::new();

        engine.add_rule(StyleRule::new(Selector::classes(&["primary", "large"]), {
            let mut m = BTreeMap::new();
            m.insert(Arc::from("border-radius"), PropValue::str("8px"));
            m
        }));

        let class_list = class_list_from_props_str("primary large");
        let class_refs: Vec<&str> = class_list.iter().map(|s| s.as_str()).collect();
        let result = engine.match_widget("Button", &class_refs, &[], &BTreeMap::new());

        assert!(result.contains_key("border-radius"));
    }
}

/// 从 WidgetView.props 中提取 class 属性值并按空格拆分为类名列表。
///
/// 返回 `Vec<String>`——调用方可通过 `iter().map(|s| s.as_str()).collect()`
/// 转换为 `Vec<&str>` 传入 `Selector::matches` 或 `SelectorEngine::match_widget`。
///
/// # 示例
///
/// ```
/// use std::collections::BTreeMap;
/// use rgui_core::view::PropValue;
/// use rgui_style::class_list_from_props;
/// let mut props = BTreeMap::new();
/// props.insert("class", PropValue::Str(std::sync::Arc::from("primary large")));
/// let classes: Vec<String> = class_list_from_props(&props);
/// assert_eq!(classes, vec!["primary".to_string(), "large".to_string()]);
/// ```
#[must_use]
pub fn class_list_from_props(props: &BTreeMap<&str, PropValue>) -> Vec<String> {
    props
        .get("class")
        .and_then(|v| {
            if let PropValue::Str(s) = v {
                Some(
                    s.split_whitespace()
                        .filter(|c| !c.is_empty())
                        .map(|c| c.to_string())
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// 测试辅助：从字符串字面量创建类列表切片。
///
/// 仅用于测试——将类名字符串按空格拆分，返回 `Vec<String>`。
#[cfg(test)]
fn class_list_from_props_str(class_str: &str) -> Vec<String> {
    class_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}
