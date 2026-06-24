//! 交互注册宿主 trait——组件通过此接口注册 hit test 回调，不依赖具体 App 实现。
//!
//! 定义在基础层（rgui_core），由框架层（rgui::App）实现。
//! 组件层（rgui-components）依赖此 trait，不依赖具体 App 实现，
//! 从而打破 `rgui-components → rgui` 的循环依赖。
//!
//! Qt 等价物：`QObject::connect()`。

use crate::coord_chain::CoordinateTransformChain;
use crate::geometry::Rect;
use crate::id::WidgetId;
use crate::widget_state::WidgetStateStore;

/// 交互注册宿主。
///
/// 组件通过此 trait 注册 hit test 回调，不依赖 `rgui::App`。
/// `Box<dyn FnMut>` 而非泛型参数，确保 object-safe。
pub trait InteractionHost {
    /// 注册一个交互回调。当命中测试命中指定 widget 且 action 匹配时触发。
    fn register_interaction(
        &mut self,
        id: WidgetId,
        bounds: Rect,
        action: &str,
        cb: Box<dyn FnMut(&str) + Send>,
    );

    /// 注册带坐标变换链的交互回调。
    fn register_interaction_with_chain(
        &mut self,
        id: WidgetId,
        bounds: Rect,
        window_to_local: CoordinateTransformChain,
        action: &str,
        cb: Box<dyn FnMut(&str) + Send>,
    );

    /// 返回 widget 状态存储的引用。
    fn widget_state_store(&self) -> &WidgetStateStore;
}
