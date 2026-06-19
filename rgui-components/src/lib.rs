//! # rgui-components
//!
//! rgui 内置组件库——从 Web Awesome (MIT) 手工翻译。
//!
//! 当前已翻译组件：
//! - `WaButton` — 按钮组件（wa-button）
//! - `WaDivider` — 分隔线组件（wa-divider）
//! - `WaCallout` — 提示/标注组件（wa-callout，原 sl-alert）

pub mod wa_avatar;
pub mod wa_badge;
pub mod wa_breadcrumb;
pub mod wa_breadcrumb_item;
pub mod wa_button;
pub mod wa_callout;
pub mod wa_card;
pub mod wa_checkbox;
pub mod wa_checkbox_group;
pub mod wa_color_picker;
pub mod wa_copy_button;
pub mod wa_details;
pub mod wa_divider;
pub mod wa_icon;
pub mod wa_input;
pub mod wa_progress_bar;
pub mod wa_progress_ring;
pub mod wa_radio;
pub mod wa_radio_group;
pub mod wa_select;
pub mod wa_skeleton;
pub mod wa_slider;
pub mod wa_spinner;
pub mod wa_switch;
pub mod wa_tag;
pub mod wa_rating;
pub mod wa_tab;
pub mod wa_tab_group;
pub mod wa_tab_panel;
pub mod wa_textarea;

pub use wa_avatar::{WaAvatar, WaAvatarMessage, WaAvatarState};
pub use wa_badge::{WaBadge, WaBadgeMessage, WaBadgeState};
pub use wa_breadcrumb::{WaBreadcrumb, WaBreadcrumbMessage, WaBreadcrumbState};
pub use wa_breadcrumb_item::{WaBreadcrumbItem, WaBreadcrumbItemMessage, WaBreadcrumbItemState};
pub use wa_button::{WaButton, WaButtonMessage, WaButtonState};
pub use wa_callout::{WaCallout, WaCalloutMessage, WaCalloutState};
pub use wa_card::{WaCard, WaCardMessage, WaCardState};
pub use wa_checkbox::{WaCheckbox, WaCheckboxMessage, WaCheckboxState};
pub use wa_copy_button::{WaCopyButton, WaCopyButtonMessage, WaCopyButtonState};
pub use wa_details::{WaDetails, WaDetailsMessage, WaDetailsState};
pub use wa_divider::{WaDivider, WaDividerMessage, WaDividerState};
pub use wa_icon::{WaIcon, WaIconMessage, WaIconState};
pub use wa_input::{WaInput, WaInputMessage, WaInputState};
pub use wa_progress_bar::{WaProgressBar, WaProgressBarMessage, WaProgressBarState};
pub use wa_progress_ring::{WaProgressRing, WaProgressRingMessage, WaProgressRingState};
pub use wa_radio::{WaRadio, WaRadioMessage, WaRadioState};
pub use wa_radio_group::{WaRadioGroup, WaRadioGroupMessage, WaRadioGroupState};
pub use wa_rating::{WaRating, WaRatingMessage, WaRatingState};
pub use wa_select::{WaSelect, WaSelectMessage, WaSelectState};
pub use wa_skeleton::{WaSkeleton, WaSkeletonMessage, WaSkeletonState};
pub use wa_slider::{WaSlider, WaSliderMessage, WaSliderState};
pub use wa_spinner::{WaSpinner, WaSpinnerMessage, WaSpinnerState};
pub use wa_switch::{WaSwitch, WaSwitchMessage, WaSwitchState};
pub use wa_tab::{WaTab, WaTabMessage, WaTabState};
pub use wa_tab_group::{WaTabGroup, WaTabGroupMessage, WaTabGroupState};
pub use wa_tab_panel::{WaTabPanel, WaTabPanelMessage, WaTabPanelState};
pub use wa_tag::{WaTag, WaTagMessage, WaTagState};
pub use wa_textarea::{WaTextarea, WaTextareaMessage, WaTextareaState};
