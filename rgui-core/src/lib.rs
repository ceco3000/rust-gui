//! # rgui-core
//!
//! rgui 框架核心类型与 trait 定义——零平台依赖。
//!
//! 本 crate 定义框架的基础抽象层，包含：
//! - 组件模型 trait（[`WidgetSpec`]）
//! - 状态管理 trait（[`PersistState`]、[`AppMessage`]）
//! - 声明式视图类型（[`WidgetView`]、[`PropValue`]）
//! - 基础几何类型（[`Rect`]、[`Size`]、[`Point`]、[`LayoutStyle`]、
//!   [`VisualStyle`]、[`TextStyle`]）
//! - Context 类型（[`ViewContext`]、[`UpdateContext`] 等）
//! - 无障碍基础类型（[`AccessibilityNode`]、[`AccessibilityRole`] 等）
//!
//! ## 设计约束
//!
//! `rgui-core` 不允许依赖任何平台相关 crate（wgpu、winit、vello、accesskit）。
//! 这是框架的最底层抽象，所有其他 crate 都依赖它。

// 模块声明
pub mod a11y;
pub mod context;
pub mod geometry;
pub mod id;
pub mod registry;
pub mod traits;
pub mod view;

// 标识符
pub use id::{WidgetId, WindowId};

// 几何类型
pub use geometry::{
    AlignContent, AlignItems, AlignSelf, BoxConstraints, FlexBasis, FlexDirection, FlexWrap,
    FontStyle, FontWeight, GridTrack, JustifyContent, LayoutDisplay, LayoutStyle, Point, Rect,
    Size, TextAlign, TextOverflow, TextStyle, Visibility, VisualStyle, WhiteSpace,
};

// 视图类型
pub use view::{Callback, Color, Key, MessageBinding, MessageHandler, PropValue, WidgetView};

// 核心 trait
pub use traits::{AppMessage, PersistState, WidgetSpec};

// Context 类型
pub use context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};

// 无障碍类型
pub use a11y::{AccessibilityAction, AccessibilityNode, AccessibilityRole, AccessibilityState};

// 注册表
pub use registry::{RegistryError, WidgetRegistry};

/// rgui-core 预导入模块。
///
/// 包含使用框架时最常用的类型和 trait。
/// 建议在 crate 根部添加 `use rgui_core::prelude::*;`。
pub mod prelude {
    pub use crate::a11y::{
        AccessibilityAction, AccessibilityNode, AccessibilityRole, AccessibilityState,
    };
    pub use crate::context::{
        AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext,
    };
    pub use crate::geometry::{
        AlignContent, AlignItems, AlignSelf, BoxConstraints, FlexBasis, FlexDirection, FlexWrap,
        FontStyle, FontWeight, GridTrack, JustifyContent, LayoutDisplay, LayoutStyle, Point, Rect,
        Size, TextAlign, TextOverflow, TextStyle, Visibility, VisualStyle, WhiteSpace,
    };
    pub use crate::id::{WidgetId, WindowId};
    pub use crate::registry::{RegistryError, WidgetRegistry};
    pub use crate::traits::{AppMessage, PersistState, WidgetSpec};
    pub use crate::view::{
        Callback, Color, Key, MessageBinding, MessageHandler, PropValue, WidgetView,
    };
}
