//! 核心 Trait 体系——AppMessage、PersistState、WidgetSpec。
//!
//! 定义源自 D0 §3。这些 trait 是框架的抽象边界，
//! 所有组件、状态管理和渲染后端均围绕它们构建。

use crate::a11y::AccessibilityNode;
use crate::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use crate::geometry::{BoxConstraints, Rect, Size};
use crate::id::WidgetId;
use crate::view::{PropValue, WidgetView};
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
    ///
    /// 默认实现委托给 [`default_measure()`](Self::default_measure)。
    /// 组件可实现此方法以自定义测量逻辑，或覆盖
    /// `default_measure()` 让派生宏生成的 `measure()` 使用自定义逻辑。
    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        ctx: &MeasureContext,
    ) -> Size {
        self.default_measure(state, constraints, ctx)
    }

    /// 默认测量实现：返回 `Size::ZERO`。
    ///
    /// 派生宏 `#[derive(WidgetSpec)]` 生成的 `measure()` 调用此方法。
    /// 使用派生宏的组件可覆盖此方法以提供自定义测量，无需手动实现 `measure()`。
    fn default_measure(
        &self,
        _state: &Self::State,
        _constraints: BoxConstraints,
        _ctx: &MeasureContext,
    ) -> Size {
        Size::ZERO
    }

    /// 绘制：将当前状态转换为绘制指令。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext);

    /// 生成无障碍节点。框架在布局后调用。
    /// 默认实现返回 `AccessibilityNode::none()`。
    fn accessibility(&self, _state: &Self::State, _ctx: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

// ============================================================================
// EventResult
// ============================================================================

/// 事件处理结果——控制事件传播与默认行为。
///
/// 组件实例的 `update()` 或事件处理器返回此枚举，
/// 框架根据结果决定是否继续传播事件或调用默认行为。
///
/// ## 变体语义（D5 §8）
///
/// | 变体 | 停止传播 | 阻止默认行为 | 说明 |
/// |------|:--------:|:----------:|------|
/// | `Handled` | ✅ | — | 事件已处理，不继续传播 |
/// | `Prevented` | — | ✅ | 阻止默认行为（如右键菜单） |
/// | `Continue(M)` | ❌ | — | 继续传播，可能携带衍生消息 |
pub enum EventResult<M> {
    /// 事件已处理，停止传播到父组件。
    Handled,
    /// 事件已阻止，不调用默认行为。
    Prevented,
    /// 继续传播，携带衍生消息。
    Continue(M),
}

// ============================================================================
// FormField
// ============================================================================

/// 表单字段验证错误。
#[derive(Debug, Clone, PartialEq)]
pub struct FormFieldError {
    pub message: String,
}

impl FormFieldError {
    #[must_use]
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for FormFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FormFieldError {}

/// 表单字段 trait。
///
/// 表单容器通过 trait bound 收集、校验、重置表单字段。
/// 替代 Web Components 的 `formAssociated` + `ElementInternals`。
///
/// ## 实现指南
///
/// 组件状态类型实现此 trait。`value()` 返回字段当前值，
/// `validate()` 检查 required/pattern 等约束，`reset()` 恢复默认值。
///
/// ## 示例
///
/// ```ignore
/// impl FormField for WaCheckboxState {
///     fn value(&self) -> PropValue {
///         PropValue::Bool(self.checked)
///     }
///
///     fn validate(&self) -> Result<(), FormFieldError> {
///         if self.required && !self.checked {
///             return Err(FormFieldError::new("此项为必填"));
///         }
///         Ok(())
///     }
///
///     fn reset(&mut self) {
///         self.checked = false;
///     }
///
///     fn field_name(&self) -> &'static str {
///         &self.name
///     }
/// }
/// ```
pub trait FormField {
    /// 返回字段的当前值。
    fn value(&self) -> PropValue;

    /// 验证当前值是否有效。
    ///
    /// 返回 `Ok(())` 表示有效，`Err` 包含错误描述。
    fn validate(&self) -> Result<(), FormFieldError>;

    /// 将字段重置为默认/初始值。
    fn reset(&mut self);

    /// 返回字段的标识名称（用于表单序列化和错误定位）。
    fn field_name(&self) -> &'static str;
}

// ============================================================================
// WidgetLifecycle
// ============================================================================

/// 组件生命周期回调 trait。
///
/// 组件可选择实现此 trait 以响应生命周期事件：
///
/// - `on_mount`：组件挂载到 widget 树时触发
/// - `on_unmount`：组件从 widget 树卸载前触发
/// - `on_reparent`：组件在树中位置变化时触发
///
/// 替代 Web Components 的 `connectedCallback`/`disconnectedCallback`。
///
/// ## 使用场景
///
/// - 订阅外部事件源（定时器、网络推送）
/// - 取消订阅、释放外部资源
/// - 响应树结构变化
///
/// ## 与 WidgetSpec 的关系
///
/// 实现此 trait 的组件应同时实现 [`WidgetSpec`]。此 trait
/// 从 WidgetSpec 独立出来以保持对象安全性，便于 WidgetTree
/// 以 `Box<dyn WidgetLifecycle>` 形式存储回调。
///
/// 所有方法都有默认空实现，组件只需覆盖关心的方法。
///
/// 定义源自 D1 §5.3。
///
/// # 示例
///
/// ```ignore
/// struct MyComponent;
///
/// impl WidgetLifecycle for MyComponent {
///     fn on_mount(&self, ctx: &mut UpdateContext) {
///         println!("组件已挂载");
///     }
///
///     fn on_unmount(&self, ctx: &mut UpdateContext) {
///         println!("组件已卸载");
///     }
/// }
/// ```
pub trait WidgetLifecycle: Send + Sync + 'static {
    /// 组件首次挂载到 widget 树时调用。
    ///
    /// 可用于订阅外部事件源、启动定时器等。
    fn on_mount(&self, _ctx: &mut UpdateContext) {}

    /// 组件从 widget 树卸载前调用。
    ///
    /// 可用于取消订阅、释放外部资源等。
    fn on_unmount(&self, _ctx: &mut UpdateContext) {}

    /// 组件在 widget 树中的位置发生变化时调用。
    ///
    /// - `old_parent`：移动前的父节点
    /// - `new_parent`：移动后的父节点
    fn on_reparent(&self, _old_parent: WidgetId, _new_parent: WidgetId) {}
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::sync::Arc;

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

    // ------------------------------------------------------------------
    // FormField 测试
    // ------------------------------------------------------------------

    /// 模拟复选框状态的测试结构体。
    #[derive(Debug, Clone)]
    struct MockCheckboxState {
        checked: bool,
        required: bool,
    }

    impl MockCheckboxState {
        fn new(checked: bool, required: bool) -> Self {
            Self { checked, required }
        }
    }

    impl FormField for MockCheckboxState {
        fn value(&self) -> PropValue {
            PropValue::Bool(self.checked)
        }

        fn validate(&self) -> Result<(), FormFieldError> {
            if self.required && !self.checked {
                return Err(FormFieldError::new("此项为必填"));
            }
            Ok(())
        }

        fn reset(&mut self) {
            self.checked = false;
        }

        fn field_name(&self) -> &'static str {
            "mock_checkbox"
        }
    }

    /// 模拟文本输入状态的测试结构体。
    #[derive(Debug, Clone)]
    struct MockInputState {
        value: String,
        default_value: String,
        required: bool,
    }

    impl MockInputState {
        fn new(value: impl Into<String>, required: bool) -> Self {
            let value = value.into();
            Self {
                default_value: value.clone(),
                value,
                required,
            }
        }
    }

    impl FormField for MockInputState {
        fn value(&self) -> PropValue {
            PropValue::Str(Arc::from(self.value.as_str()))
        }

        fn validate(&self) -> Result<(), FormFieldError> {
            if self.required && self.value.is_empty() {
                return Err(FormFieldError::new("此项为必填"));
            }
            Ok(())
        }

        fn reset(&mut self) {
            self.value = self.default_value.clone();
        }

        fn field_name(&self) -> &'static str {
            "mock_input"
        }
    }

    #[test]
    fn form_field_value_returns_correct_propvalue() {
        let field = MockCheckboxState::new(true, false);
        assert_eq!(field.value(), PropValue::Bool(true));

        let field = MockCheckboxState::new(false, true);
        assert_eq!(field.value(), PropValue::Bool(false));
    }

    #[test]
    fn form_field_validate_passes_when_not_required() {
        let field = MockCheckboxState::new(false, false);
        assert!(field.validate().is_ok());
    }

    #[test]
    fn form_field_validate_fails_when_required_and_empty() {
        let field = MockCheckboxState::new(false, true);
        let result = field.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "此项为必填");
    }

    #[test]
    fn form_field_reset_restores_default() {
        let mut field = MockCheckboxState::new(true, false);
        assert_eq!(field.value(), PropValue::Bool(true));
        field.reset();
        assert_eq!(field.value(), PropValue::Bool(false));
    }

    #[test]
    fn form_field_name_returns_static_str() {
        let field = MockCheckboxState::new(false, false);
        assert_eq!(field.field_name(), "mock_checkbox");
    }

    #[test]
    fn form_field_input_value_returns_string() {
        let field = MockInputState::new("hello", false);
        assert_eq!(field.value(), PropValue::Str(Arc::from("hello")));
    }

    #[test]
    fn form_field_input_validate_empty_required() {
        let field = MockInputState::new("", true);
        let result = field.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "此项为必填");
    }

    #[test]
    fn form_field_input_validate_nonempty_passes() {
        let field = MockInputState::new("hello", true);
        assert!(field.validate().is_ok());
    }

    #[test]
    fn form_field_input_reset_to_default() {
        let mut field = MockInputState::new("default", false);
        field.value = "modified".into();
        field.reset();
        assert_eq!(field.value(), PropValue::Str(Arc::from("default")));
    }

    #[test]
    fn form_field_error_display() {
        let err = FormFieldError::new("invalid");
        assert_eq!(err.to_string(), "invalid");
    }

    #[test]
    fn form_field_error_partial_eq() {
        let a = FormFieldError::new("error");
        let b = FormFieldError::new("error");
        let c = FormFieldError::new("different");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ------------------------------------------------------------------
    // WidgetLifecycle 测试
    // ------------------------------------------------------------------

    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 模拟实现了 WidgetLifecycle 的组件，用原子计数器追踪回调调用次数。
    struct MockLifecycleComponent {
        mount_count: AtomicUsize,
        unmount_count: AtomicUsize,
        reparent_count: AtomicUsize,
        last_old_parent: Mutex<Option<WidgetId>>,
        last_new_parent: Mutex<Option<WidgetId>>,
    }

    impl MockLifecycleComponent {
        fn new() -> Self {
            Self {
                mount_count: AtomicUsize::new(0),
                unmount_count: AtomicUsize::new(0),
                reparent_count: AtomicUsize::new(0),
                last_old_parent: Mutex::new(None),
                last_new_parent: Mutex::new(None),
            }
        }
    }

    impl WidgetLifecycle for MockLifecycleComponent {
        fn on_mount(&self, _ctx: &mut UpdateContext) {
            self.mount_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_unmount(&self, _ctx: &mut UpdateContext) {
            self.unmount_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_reparent(&self, old_parent: WidgetId, new_parent: WidgetId) {
            self.reparent_count.fetch_add(1, Ordering::SeqCst);
            *self.last_old_parent.lock().unwrap() = Some(old_parent);
            *self.last_new_parent.lock().unwrap() = Some(new_parent);
        }
    }

    #[test]
    fn lifecycle_default_impls_are_noops() {
        // 默认实现不应 panic
        struct DefaultComponent;
        impl WidgetLifecycle for DefaultComponent {}

        let comp = DefaultComponent;
        let mut ctx = UpdateContext::new();
        comp.on_mount(&mut ctx);
        comp.on_unmount(&mut ctx);
        comp.on_reparent(WidgetId::from_u64(1), WidgetId::from_u64(2));
    }

    #[test]
    fn lifecycle_on_mount_increments_counter() {
        let comp = MockLifecycleComponent::new();
        let mut ctx = UpdateContext::new();
        assert_eq!(comp.mount_count.load(Ordering::SeqCst), 0);
        comp.on_mount(&mut ctx);
        assert_eq!(comp.mount_count.load(Ordering::SeqCst), 1);
        comp.on_mount(&mut ctx);
        assert_eq!(comp.mount_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn lifecycle_on_unmount_increments_counter() {
        let comp = MockLifecycleComponent::new();
        let mut ctx = UpdateContext::new();
        assert_eq!(comp.unmount_count.load(Ordering::SeqCst), 0);
        comp.on_unmount(&mut ctx);
        assert_eq!(comp.unmount_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn lifecycle_on_reparent_records_parents() {
        let comp = MockLifecycleComponent::new();
        let old = WidgetId::from_u64(10);
        let new = WidgetId::from_u64(20);
        assert_eq!(comp.reparent_count.load(Ordering::SeqCst), 0);
        comp.on_reparent(old, new);
        assert_eq!(comp.reparent_count.load(Ordering::SeqCst), 1);
        assert_eq!(*comp.last_old_parent.lock().unwrap(), Some(old));
        assert_eq!(*comp.last_new_parent.lock().unwrap(), Some(new));
    }

    #[test]
    fn lifecycle_boxed_trait_object() {
        // 验证对象安全性：可存储为 Box<dyn WidgetLifecycle>
        let comp = MockLifecycleComponent::new();
        let boxed: Box<dyn WidgetLifecycle> = Box::new(comp);
        let mut ctx = UpdateContext::new();
        boxed.on_mount(&mut ctx);
        boxed.on_unmount(&mut ctx);
        boxed.on_reparent(WidgetId::from_u64(1), WidgetId::from_u64(2));
    }

    #[test]
    fn lifecycle_arc_trait_object() {
        // 验证可通过 Arc 共享
        let comp = Arc::new(MockLifecycleComponent::new());
        let mut ctx = UpdateContext::new();
        comp.on_mount(&mut ctx);
        assert_eq!(comp.mount_count.load(Ordering::SeqCst), 1);

        let comp2 = Arc::clone(&comp);
        comp2.on_mount(&mut ctx);
        assert_eq!(comp.mount_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn lifecycle_reparent_called_multiple_times() {
        let comp = MockLifecycleComponent::new();
        comp.on_reparent(WidgetId::from_u64(1), WidgetId::from_u64(2));
        comp.on_reparent(WidgetId::from_u64(2), WidgetId::from_u64(3));
        comp.on_reparent(WidgetId::from_u64(3), WidgetId::from_u64(4));
        assert_eq!(comp.reparent_count.load(Ordering::SeqCst), 3);
        assert_eq!(
            *comp.last_old_parent.lock().unwrap(),
            Some(WidgetId::from_u64(3))
        );
        assert_eq!(
            *comp.last_new_parent.lock().unwrap(),
            Some(WidgetId::from_u64(4))
        );
    }

    #[test]
    fn lifecycle_mount_unmount_independent_counters() {
        let comp = MockLifecycleComponent::new();
        let mut ctx = UpdateContext::new();
        comp.on_mount(&mut ctx);
        comp.on_mount(&mut ctx);
        comp.on_unmount(&mut ctx);
        assert_eq!(comp.mount_count.load(Ordering::SeqCst), 2);
        assert_eq!(comp.unmount_count.load(Ordering::SeqCst), 1);
    }
}
