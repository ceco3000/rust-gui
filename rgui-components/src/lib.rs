//! # rgui-components
//!
//! rgui 内置组件库——从 Web Awesome (MIT) 手工翻译。
//!
//! 当前已翻译组件：
//! - `WaButton` — 按钮组件（wa-button）
//! - `WaDivider` — 分隔线组件（wa-divider）

pub mod wa_avatar;
pub mod wa_badge;
pub mod wa_breadcrumb;
pub mod wa_breadcrumb_item;
pub mod wa_button;
pub mod wa_card;
pub mod wa_divider;
pub mod wa_icon;
pub mod wa_spinner;

pub use wa_avatar::{WaAvatar, WaAvatarMessage, WaAvatarState};
pub use wa_badge::{WaBadge, WaBadgeMessage, WaBadgeState};
pub use wa_breadcrumb::{WaBreadcrumb, WaBreadcrumbMessage, WaBreadcrumbState};
pub use wa_breadcrumb_item::{WaBreadcrumbItem, WaBreadcrumbItemMessage, WaBreadcrumbItemState};
pub use wa_button::{WaButton, WaButtonMessage, WaButtonState};
pub use wa_card::{WaCard, WaCardMessage, WaCardState};
pub use wa_divider::{WaDivider, WaDividerMessage, WaDividerState};
pub use wa_icon::{WaIcon, WaIconMessage, WaIconState};
pub use wa_spinner::{WaSpinner, WaSpinnerMessage, WaSpinnerState};
