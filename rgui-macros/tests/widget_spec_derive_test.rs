//! WidgetSpec derive 宏集成测试。
//!
//! 验证：
//! - name() 返回正确的类型名称
//! - update() 为空操作（调用后状态不变）
//! - measure() 返回 Size::ZERO
//! - accessibility() 使用 trait 默认实现（返回 AccessibilityNode::none()）
//! - view() 和 paint() 通过固有方法委托正确工作

use std::sync::atomic::{AtomicBool, Ordering};

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
use rgui_core::view::WidgetView;
use rgui_macros::{AppMessage, PersistState, WidgetSpec};

// ===== 测试用状态类型 =====

#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
struct TestState {
    value: i32,
}

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum TestMessage {
    Increment,
}

// ===== 测试用组件 =====

/// 最基本的组件——使用默认 name（类型名）。
#[derive(WidgetSpec)]
#[widget(state = TestState, message = TestMessage)]
struct BasicWidget;

impl BasicWidget {
    fn __widget_view(&self, state: &TestState, _ctx: &ViewContext) -> WidgetView<TestMessage> {
        let label = format!("Value: {}", state.value);
        WidgetView::new("Label").prop("text", rgui_core::view::PropValue::str(label.as_str()))
    }

    fn __widget_paint(&self, _state: &TestState, _bounds: Rect, _ctx: &mut PaintContext) {
        // 空实现
    }
}

/// 自定义名称的组件。
#[derive(WidgetSpec)]
#[widget(
    state = TestState,
    message = TestMessage,
    name = "my_app::CustomWidget"
)]
struct CustomNameWidget;

impl CustomNameWidget {
    fn __widget_view(&self, _state: &TestState, _ctx: &ViewContext) -> WidgetView<TestMessage> {
        WidgetView::new("Empty")
    }

    fn __widget_paint(&self, _state: &TestState, _bounds: Rect, _ctx: &mut PaintContext) {
        // 空实现
    }
}

/// 可追踪 paint 调用的组件。
#[derive(WidgetSpec)]
#[widget(state = TestState, message = TestMessage)]
struct PaintTrackWidget {
    paint_called: AtomicBool,
}

impl PaintTrackWidget {
    fn __widget_view(&self, _state: &TestState, _ctx: &ViewContext) -> WidgetView<TestMessage> {
        WidgetView::new("Empty")
    }

    fn __widget_paint(&self, _state: &TestState, _bounds: Rect, _ctx: &mut PaintContext) {
        self.paint_called.store(true, Ordering::SeqCst);
    }
}

// ===== 测试 =====

#[test]
fn widget_spec_name_default() {
    assert_eq!(BasicWidget.name(), "BasicWidget");
}

#[test]
fn widget_spec_name_custom() {
    assert_eq!(CustomNameWidget.name(), "my_app::CustomWidget");
}

#[test]
fn widget_spec_update_is_noop() {
    let comp = BasicWidget;
    let mut state = TestState { value: 42 };
    let mut ctx = UpdateContext::default();

    comp.update(TestMessage::Increment, &mut state, &mut ctx);
    // update 是空操作，state 不应改变
    assert_eq!(state.value, 42);
}

#[test]
fn widget_spec_measure_returns_zero() {
    let comp = BasicWidget;
    let state = TestState { value: 0 };
    let constraints = BoxConstraints::tight(Size::new(100.0, 100.0));
    let ctx = MeasureContext::default();

    let size = comp.measure(&state, constraints, &ctx);
    assert_eq!(size, Size::ZERO);
}

#[test]
fn widget_spec_accessibility_returns_none() {
    let comp = BasicWidget;
    let state = TestState { value: 10 };
    let ctx = AccessContext::new(Rect::ZERO);

    let node = comp.accessibility(&state, &ctx);
    assert_eq!(node, AccessibilityNode::none());
}

#[test]
fn widget_spec_view_delegates_to_inherent_method() {
    let comp = BasicWidget;
    let state = TestState { value: 7 };
    let ctx = ViewContext::new(Size::new(800.0, 600.0));

    let view = comp.view(&state, &ctx);
    // 验证 view() 委托到 __widget_view()
    assert_eq!(view.widget_type, "Label");
}

#[test]
fn widget_spec_paint_delegates_to_inherent_method() {
    let comp = PaintTrackWidget {
        paint_called: AtomicBool::new(false),
    };
    let state = TestState { value: 0 };
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let mut ctx = PaintContext::new(bounds);

    // 调用前确认 paint_called 为 false
    assert!(!comp.paint_called.load(Ordering::SeqCst));

    comp.paint(&state, bounds, &mut ctx);

    // paint() 应委托到 __widget_paint()，将标志设为 true
    assert!(comp.paint_called.load(Ordering::SeqCst));
}

#[test]
fn widget_spec_associated_types_correct() {
    // 编译时验证关联类型的正确性
    fn _check_types() {
        let _s: <BasicWidget as WidgetSpec>::State = TestState::default();
        let _m: <BasicWidget as WidgetSpec>::Message = TestMessage::Increment;
        let _: <CustomNameWidget as WidgetSpec>::State = TestState::default();
        let _: <CustomNameWidget as WidgetSpec>::Message = TestMessage::Increment;
    }
}
