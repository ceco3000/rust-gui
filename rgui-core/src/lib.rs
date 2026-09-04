//! # rgui-core
//!
//! rgui 框架逻辑核心——唯一逻辑层。吸收 state/layout/components/a11y_tree。
//!
//! ## 设计约束
//!
//! `rgui-core` 是框架最底层抽象，**零平台/零 GPU/零 cssparser 依赖**：
//! - 不允许依赖任何平台相关 crate（wgpu、winit、vello、accesskit、cssparser）。
//! - 不持有 GPU 资源类型（`GlyphKey`/`PathTessellation` 等在 `rgui-render`）。
//! - 所有其他 crate（render/platform/style/macros）都可依赖它；反向禁止。
//!
//! 本 crate 定义框架基础抽象层：组件模型 trait（[`WidgetSpec`]）、状态管理 trait
//! （[`PersistState`]、[`AppMessage`]）、声明式视图类型（[`WidgetView`]、[`PropValue`]）、
//! 几何类型（[`Rect`]、[`Size`]、[`Point`]）、Context 类型、无障碍基础类型。
//!
//! D3 阶段 0：仅建立模块骨架 + 契约 trait（签名占位），不实现业务逻辑。

// 模块声明
pub mod a11y;
pub mod a11y_tree;
pub mod color;
pub mod components;
pub mod context;
pub mod coordinator;
pub mod geometry;
pub mod id;
pub mod layout;
pub mod locale;
pub mod message;
pub mod registry;
pub mod state;
pub mod style;
pub mod traits;
pub mod view;
pub mod widget_state;

// 标识符
pub use id::{NodeHandle, WidgetId, WindowId};

// 几何类型（核心契约所需；完整样式枚举在实现阶段补全）
pub use geometry::{BoxConstraints, Point, Rect, Size};

// 视图类型
pub use view::{Callback, Color, Key, MessageBinding, MessageHandler, PropValue, WidgetView};

// 核心 trait
pub use traits::{
    AppMessage, EventResult, PersistState, WidgetSpec,
};

// 内置消息类型
pub use message::NoopMsg;

// Context 类型
pub use context::{
    AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext,
};

// 无障碍基础类型
pub use a11y::{AccessibilityAction, AccessibilityNode, AccessibilityRole, AccessibilityState};

// 状态管理子模块（由 rgui-state 迁入）
pub use state::{
    apply_patch, diff, InstanceState, Patch, SchemaMigration, Snapshot, Snapshotter, StateStore,
    StoreBinding, Subscription, SubscriptionLifetime,
};

// 布局子模块（由 rgui-layout 迁入）
pub use layout::{LayoutEngine, LayoutNode, LayoutResult, LayoutStyle};

// 组件子模块（由 rgui-components 迁入，统一 Tier 1 WidgetSpec）
pub use components::*;

// 无障碍树子模块（由 rgui-a11y/tree.rs 迁入）
pub use a11y_tree::AccessibilityTree;

// 样式系统子模块（由 rgui-style 并入，顶层通配导出）
pub use style::*;

// 关键类型契约（顶层导出，保持 facade `use rgui_core::*` 兼容性）
pub mod prelude {
    pub use crate::a11y::*;
    pub use crate::geometry::*;
    pub use crate::traits::*;
    pub use crate::view::*;
    pub use crate::context::*;
    pub use crate::id::*;
    pub use crate::message::*;
    pub use crate::state::*;
    pub use crate::layout::*;
    pub use crate::components::*;
    pub use crate::a11y_tree::*;
}
