//! # rgui-components
//!
//! rgui 内置组件库——从 Web Awesome (MIT) 手工翻译。
//!
//! 当前已翻译组件：
//! - `WaAccordion` — 手风琴容器（wa-accordion）
//! - `WaAccordionItem` — 手风琴面板项（wa-accordion-item）

pub mod wa_accordion;
pub mod wa_accordion_item;

pub use wa_accordion::{WaAccordion, WaAccordionMessage, WaAccordionState};
pub use wa_accordion_item::{WaAccordionItem, WaAccordionItemMessage, WaAccordionItemState};
