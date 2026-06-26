//! # rgui-components
//!
//! rgui 内置组件库——从 Web Awesome (MIT) 手工翻译。
//!
//! ## 组件清单
//! - Accordion / AccordionItem: Tier 2 声明式容器（.rgui + .rhai）

pub mod accordion_interactive;
pub mod wa_badge;

pub use wa_badge::{WaBadge, WaBadgeMessage, WaBadgeState};
