//! # rgui-style —— 选择器引擎、主题变量、样式合并、.rgss 解析、热重载、属性映射、CSS 变量

pub mod css_functions;
pub mod merger;
pub mod parser;
pub mod property_map;
pub mod selector;
pub mod theme;
pub mod variable;

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

pub use merger::StyleMerger;
pub use parser::{ParseError, parse_rgss};
pub use property_map::{
    PropertyCategory, PropertyMeta, ResolvedStyles, category_of, is_valid_property,
    property_meta_table, resolve_properties,
};
pub use selector::{
    CombinatorKind, MediaCondition, Selector, SelectorEngine, Specificity, StyleRule, breakpoints,
    class_list_from_props,
};
pub use theme::{ColorScheme, Theme, ThemeVariables};
pub use variable::{VariableTable, extract_variables_from_rules, parse_var_reference};

#[cfg(feature = "hot-reload")]
pub use hot_reload::{HotReloadError, StyleChange, StyleHotReload};
