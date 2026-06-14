//! # rgui-style —— 选择器引擎、主题变量、样式合并、.rgss 解析、热重载

pub mod merger;
pub mod parser;
pub mod selector;
pub mod theme;

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

pub use merger::StyleMerger;
pub use parser::{ParseError, parse_rgss};
pub use selector::{
    CombinatorKind, MediaCondition, Selector, SelectorEngine, Specificity, StyleRule, breakpoints,
};
pub use theme::{ColorScheme, Theme, ThemeVariables};

#[cfg(feature = "hot-reload")]
pub use hot_reload::{HotReloadError, StyleChange, StyleHotReload};
