//! 测试辅助工具——TestHarness。
//!
//! 提供快速构造测试环境的辅助结构体，用于组件单元测试和集成测试。
//! 定义源自 D9 §9。

use rgui_core::id::WidgetId;
use rgui_core::registry::WidgetRegistry;
use rgui_core::traits::WidgetSpec;

use crate::store::StateStore;

/// 测试 Harness：快速构造测试环境。
///
/// 封装 [`StateStore`] 和 [`WidgetRegistry`]，提供简化的 widget 挂载与
/// 测试操作 API。
///
/// # 示例
///
/// ```ignore
/// let mut harness = TestHarness::new();
/// let btn_id = harness.mount(&button, ButtonState::default());
/// harness.dispatch(Event::MouseDown { ... });
/// ```
pub struct TestHarness {
    /// 状态存储。
    pub state_store: StateStore,
    /// Widget 注册表。
    pub registry: WidgetRegistry,
}

impl TestHarness {
    /// 创建空的测试环境。
    #[must_use]
    pub fn new() -> Self {
        Self {
            state_store: StateStore::new(),
            registry: WidgetRegistry::new(),
        }
    }

    /// 注册并挂载一个 widget。
    ///
    /// 分配新的 [`WidgetId`]，在注册表中注册 widget 名称，
    /// 并将持久状态存入 [`StateStore`]。
    ///
    /// # Panics
    ///
    /// 如果同名 widget 已注册。
    pub fn mount<W: WidgetSpec>(&mut self, spec: &W, state: W::State) -> WidgetId {
        let id = self.state_store.allocate_id();
        self.registry
            .register(spec.name())
            .expect("widget 注册失败：同名 widget 已存在");
        self.state_store.insert_persistent(id, Box::new(state));
        id
    }

    /// 返回当前挂载的 widget 数量。
    #[must_use]
    pub fn widget_count(&self) -> usize {
        self.state_store.widget_count()
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::{
        AccessibilityNode,
        context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext},
        geometry::{BoxConstraints, Rect, Size},
        traits::{AppMessage, PersistState, WidgetSpec},
        view::WidgetView,
    };
    use std::any::Any;

    // ---- 用于测试的最小 WidgetSpec 实现 ----

    #[derive(Debug, Clone, PartialEq)]
    #[allow(dead_code)]
    enum TestMsg {
        Noop,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            "noop"
        }
    }

    #[derive(Debug, Clone, serde::Serialize)]
    struct TestWidgetState {
        label: String,
    }

    impl Default for TestWidgetState {
        fn default() -> Self {
            Self {
                label: "test".into(),
            }
        }
    }

    impl PersistState for TestWidgetState {
        fn schema_name() -> &'static str {
            "test_widget_state"
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

    struct TestWidget {
        name: &'static str,
    }

    impl TestWidget {
        const fn new(name: &'static str) -> Self {
            Self { name }
        }
    }

    impl WidgetSpec for TestWidget {
        type State = TestWidgetState;
        type Message = TestMsg;

        fn name(&self) -> &'static str {
            self.name
        }

        fn view(&self, _state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
            WidgetView::new(self.name)
        }

        fn update(&self, _msg: Self::Message, _state: &mut Self::State, _ctx: &mut UpdateContext) {}

        fn measure(
            &self,
            _state: &Self::State,
            _constraints: BoxConstraints,
            _ctx: &MeasureContext,
        ) -> Size {
            Size::new(100.0, 40.0)
        }

        fn paint(&self, _state: &Self::State, _bounds: Rect, _ctx: &mut PaintContext) {}

        fn accessibility(&self, _state: &Self::State, _ctx: &AccessContext) -> AccessibilityNode {
            AccessibilityNode::none()
        }
    }

    // ---- 测试用例 ----

    #[test]
    fn new_creates_empty_harness() {
        let harness = TestHarness::new();
        assert_eq!(harness.widget_count(), 0);
        assert!(harness.registry.is_empty());
    }

    #[test]
    fn default_equals_new() {
        let h1 = TestHarness::new();
        let h2 = TestHarness::default();
        assert_eq!(h1.widget_count(), h2.widget_count());
    }

    #[test]
    fn mount_registers_and_returns_id() {
        let mut harness = TestHarness::new();
        let widget = TestWidget::new("test_widget");
        let state = TestWidgetState::default();

        let id = harness.mount(&widget, state);

        assert_eq!(harness.widget_count(), 1);
        assert!(harness.registry.contains(widget.name()));
        // 验证 WidgetId 有效
        assert!(id.as_u64() > 0);
    }

    #[test]
    fn mount_multiple_widgets_generates_unique_ids() {
        let mut harness = TestHarness::new();
        let w1 = TestWidget::new("widget_a");
        let w2 = TestWidget::new("widget_b");
        let w3 = TestWidget::new("widget_c");

        let id1 = harness.mount(&w1, TestWidgetState::default());
        let id2 = harness.mount(&w2, TestWidgetState::default());
        let id3 = harness.mount(&w3, TestWidgetState::default());

        assert_eq!(harness.widget_count(), 3);
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    #[should_panic(expected = "同名 widget")]
    fn mount_duplicate_name_panics() {
        let mut harness = TestHarness::new();
        let w1 = TestWidget::new("same_name");
        let w2 = TestWidget::new("same_name");

        harness.mount(&w1, TestWidgetState::default());
        harness.mount(&w2, TestWidgetState::default());
    }

    #[test]
    fn three_line_creation() {
        // 验收标准：3 行创建测试环境
        let mut harness = TestHarness::new();
        let widget = TestWidget::new("btn");
        let btn_id = harness.mount(&widget, TestWidgetState::default());

        assert_eq!(harness.widget_count(), 1);
        assert!(btn_id.as_u64() > 0);
    }
}
