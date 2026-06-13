//! 核心 Trait 体系——AppMessage、PersistState、WidgetSpec。
//!
//! 定义源自 D0 §3。这些 trait 是框架的抽象边界，
//! 所有组件、状态管理和渲染后端均围绕它们构建。

use crate::a11y::AccessibilityNode;
use crate::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use crate::geometry::{BoxConstraints, Rect, Size};
use crate::view::WidgetView;
use std::any::Any;
use std::fmt;

// ============================================================================
// AppMessage
// ============================================================================

/// 组件产生的消息类型。
///
/// 约束：`'static`、可跨线程传递、可调试、可克隆。
/// 推荐使用 `#[derive(AppMessage)]` 派生宏自动生成。
///
/// 定义源自 D0 §3.4。
pub trait AppMessage: Send + Sync + 'static + fmt::Debug + Clone {
    /// 消息名称（用于调试和日志）。
    fn message_name(&self) -> &'static str;
}

// ============================================================================
// PersistState
// ============================================================================

/// 可持久化的业务状态。
///
/// ## 设计约束（D0 §3.3）
///
/// - 不允许持有 GPU 资源句柄（纹理 ID、Buffer 引用）
/// - 不允许持有平台句柄（窗口 ID）
/// - 不允许持有文件描述符
/// - 这些资源属于实例态和缓存态，由框架统一持有
///
/// 定义源自 D0 §3.3。
pub trait PersistState: erased_serde::Serialize + Send + Sync + 'static {
    /// 状态模式的唯一名称（用于快照迁移匹配）。
    fn schema_name() -> &'static str
    where
        Self: Sized;

    /// 状态模式的版本号（用于快照迁移）。
    fn schema_version() -> u32
    where
        Self: Sized;

    /// 将自身借用为 `&dyn Any`（用于类型擦除后的状态访问）。
    fn as_any(&self) -> &dyn Any;

    /// 将自身借用为 `&mut dyn Any`。
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ============================================================================
// WidgetSpec
// ============================================================================

/// 组件规范 trait。
///
/// 每个 UI 组件实现此 trait。框架通过 `view()` 获取声明式视图，
/// 通过 `update()` 处理用户交互，通过 `measure()` 计算布局，
/// 通过 `paint()` 生成绘制指令。
///
/// ## 调用时序（D0 §6）
///
/// 1. `update()` —— 事件分发，可能修改状态
/// 2. `measure()` —— 仅 dirty widget 子树重新布局
/// 3. `accessibility()` —— 生成无障碍节点
/// 4. `view()` —— 仅 dirty widget 重新生成视图
/// 5. `paint()` —— 生成场景图绘制指令
///
/// ## 派生宏
///
/// 框架将提供 `#[derive(WidgetSpec)]` 派生宏，自动为 `accessibility()`
/// 生成返回 `AccessibilityNode::none()` 的默认实现。
///
/// 定义源自 D0 §3.2。
pub trait WidgetSpec: Send + Sync + 'static {
    /// 组件持有的业务状态类型。
    type State: PersistState;

    /// 组件产生的消息类型。
    type Message: AppMessage;

    /// 组件的唯一名称（用于调试、注册、序列化）。
    fn name(&self) -> &'static str;

    /// 从持久状态派生声明式视图。应为纯函数。
    fn view(&self, state: &Self::State, ctx: &ViewContext) -> WidgetView<Self::Message>;

    /// 处理来自 UI 的消息。只能修改自身的持久状态。
    fn update(&self, msg: Self::Message, state: &mut Self::State, ctx: &mut UpdateContext);

    /// 纯测量：根据约束计算组件期望尺寸。不写状态。
    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        ctx: &MeasureContext,
    ) -> Size;

    /// 绘制：将当前状态转换为绘制指令。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);

    /// 生成无障碍节点。框架在布局后调用。
    /// 默认实现返回 `AccessibilityNode::none()`。
    fn accessibility(&self, _state: &Self::State, _ctx: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    /// 用于测试的简单消息类型。
    #[derive(Debug, Clone, PartialEq)]
    enum TestMessage {
        Clicked,
        TextChanged(String),
    }

    impl AppMessage for TestMessage {
        fn message_name(&self) -> &'static str {
            match self {
                Self::Clicked => "clicked",
                Self::TextChanged(_) => "text_changed",
            }
        }
    }

    /// 用于测试的持久状态类型。
    #[derive(serde::Serialize)]
    struct TestState {
        count: i32,
    }

    impl PersistState for TestState {
        fn schema_name() -> &'static str {
            "test_state"
        }

        fn schema_version() -> u32 {
            1
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn app_message_name() {
        let msg = TestMessage::Clicked;
        assert_eq!(msg.message_name(), "clicked");
    }

    #[test]
    fn app_message_clone_is_equal() {
        let msg = TestMessage::Clicked;
        assert_eq!(msg, msg.clone());
    }

    #[test]
    fn app_message_text_changed_debug() {
        let msg = TestMessage::TextChanged("hello".into());
        let debug_str = format!("{msg:?}");
        assert!(debug_str.contains("hello"));
    }

    #[test]
    fn persist_state_schema_name() {
        assert_eq!(TestState::schema_name(), "test_state");
    }

    #[test]
    fn persist_state_schema_version() {
        assert_eq!(TestState::schema_version(), 1);
    }

    #[test]
    fn persist_state_as_any_type_id() {
        let state = TestState { count: 42 };
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<TestState>());
    }
}
