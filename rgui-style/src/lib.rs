//! # rgui-style —— 选择器引擎、主题变量、样式合并、.rgss 解析

pub mod selector;
pub mod theme;
pub mod merger;
pub mod parser;

pub use selector::{CombinatorKind, Selector, SelectorEngine, Specificity, StyleRule};
pub use theme::{ColorScheme, Theme, ThemeVariables};
pub use merger::StyleMerger;
pub use parser::{parse_rgss, ParseError};
