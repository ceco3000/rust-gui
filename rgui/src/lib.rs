//! # rgui——Rust GUI 框架

#![allow(ambiguous_glob_reexports)]

pub mod app;
pub mod error_boundary;
pub mod interactive;
pub mod paint_factory;
pub mod render;
pub mod widget_node;
pub mod widget_state;

pub use app::{App, AppConfig};
#[cfg(feature = "devtools")]
pub use app::run_simple_app;
pub use rgui_a11y::*;
pub use rgui_components::*;
pub use rgui_core::*;
pub use rgui_layout::*;
pub use rgui_macros::{AppMessage, WidgetSpec, html};
pub use rgui_platform::*;
pub use rgui_render::*;
pub use rgui_state::TestHarness;
pub use rgui_state::*;
pub use rgui_style::*;
pub use widget_node::WidgetNode;

pub mod prelude {
    pub use crate::app::{App, AppConfig};
    pub use rgui_a11y::*;
    pub use rgui_components::*;
    pub use rgui_core::prelude::*;
    pub use rgui_layout::*;
    pub use rgui_macros::{AppMessage, WidgetSpec, html};
    pub use rgui_platform::*;
    pub use rgui_render::*;
    pub use rgui_state::*;
    pub use rgui_style::*;
}
