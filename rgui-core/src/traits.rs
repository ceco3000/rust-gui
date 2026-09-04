//! 核心契约 trait：`WidgetSpec` / `AppMessage` / `PersistState` 及辅助类型。
//!
//! D3 阶段 0：签名按 greenfield 基准 §B.1 准确定义占位，方法体为最小 stub（`todo!()` 或空实现），
//! 不实现业务逻辑。**契约严格对齐 §B.1**：measure 用 `BoxConstraints`+`MeasureContext`，
//! `EventResult` 为 `{Handled, Prevented, Continue(M)}`，无文档未定义的派生 trait。

use crate::a11y::AccessibilityNode;
use crate::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use crate::geometry::{BoxConstraints, Rect, Size};
use crate::view::WidgetView;
use std::any::Any;
use std::fmt;

/// 应用消息。所有跨边界传递的消息必须满足此 trait。
pub trait AppMessage: Send + Sync + 'static + fmt::Debug + Clone {
    /// 稳定的消息名称（用于日志/调试/序列化）。
    fn message_name(&self) -> &'static str;
}

/// 持久化状态。组件/应用状态必须满足此 trait 以便快照与恢复。
///
/// D3 占位：`erased_serde::Serialize` 超级绑定在实现阶段（D6/D7 引入序列化时）按契约补全，
/// 当前仅保留 `Any` 上下转型能力作为最小可用签名。
pub trait PersistState: Send + Sync + 'static {
    /// Schema 稳定名称。
    fn schema_name() -> &'static str
    where
        Self: Sized;
    /// Schema 版本号（迁移用）。
    fn schema_version() -> u32
    where
        Self: Sized;
    /// 类型擦除的不可变引用（用于向下转型）。
    fn as_any(&self) -> &dyn Any;
    /// 类型擦除的可变引用（用于向下转型）。
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// 组件规格。统一 Tier 1 WidgetSpec 定义（greenfield §B.1）。
///
/// 一个 widget 由关联状态类型与消息类型定义，并提供 view/update/measure/paint 生命周期。
///
/// ```ignore
/// #[derive(WidgetSpec)]
/// struct MyWidget;
/// ```
pub trait WidgetSpec: Send + Sync + 'static {
    /// 组件状态。
    type State: PersistState;
    /// 组件消息。
    type Message: AppMessage;

    /// 稳定名称。
    fn name(&self) -> &'static str;

    /// 构建视图树（声明式）。
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;

    /// 处理消息并更新状态。
    fn update(&self, msg: Self::Message, state: &mut Self::State, ctx: &mut UpdateContext);

    /// 测量尺寸。
    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        ctx: &MeasureContext,
    ) -> Size;

    /// 绘制。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);

    /// 组件是否可获焦（D12：Tab 导航/焦点切换）。默认否；组件可覆盖为 `true`。
    fn focusable(&self) -> bool {
        false
    }

    /// 无障碍信息（默认无）。
    fn accessibility(&self, _s: &Self::State, _c: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

/// 事件传播结果（greenfield §B.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult<M> {
    /// 事件已处理，停止传播。
    Handled,
    /// 事件被阻止（防止默认行为），但仍需继续传播。
    Prevented,
    /// 继续传播，并携带一条派生消息 `M`。
    Continue(M),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::PropValue;

    #[derive(Debug, Clone)]
    struct NoopMsg;
    impl AppMessage for NoopMsg {
        fn message_name(&self) -> &'static str {
            "noop"
        }
    }

    #[derive(Debug)]
    struct DummyState;
    impl PersistState for DummyState {
        fn schema_name() -> &'static str {
            "dummy"
        }
        fn schema_version() -> u32 {
            0
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn app_message_name_returns_static_str() {
        let m = NoopMsg;
        assert_eq!(m.message_name(), "noop");
    }

    #[test]
    fn persist_state_schema_is_stable() {
        assert_eq!(DummyState::schema_name(), "dummy");
        assert_eq!(DummyState::schema_version(), 0);
    }

    #[test]
    fn persist_state_as_any_downcast() {
        let s = DummyState;
        let any: &dyn Any = s.as_any();
        assert!(any.is::<DummyState>());
    }

    #[test]
    fn event_result_moves_message() {
        let r = EventResult::Continue(NoopMsg);
        match r {
            EventResult::Continue(_) => {}
            _ => panic!("expected Continue"),
        }
    }
}
