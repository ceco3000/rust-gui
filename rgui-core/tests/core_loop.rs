//! 核心循环最小闭环集成测试（TDD RED 起点）。
//!
//! 目标：框架宿主 `Coordinator<W: WidgetSpec>` 能驱动组件完成
//! 「状态变化 → 视图更新 → 重绘」的 update→view 闭环。
//!
//! 测试组件 `Counter`（测试专用）：
//! - State = CountState(u32)
//! - Message = Increment
//! - view() 把当前 count 编码进 WidgetView.props（PropValue::Int）
//! - update(Increment) → count += 1

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{MeasureContext, UpdateContext, ViewContext};
use rgui_core::coordinator::Coordinator;
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use std::any::Any;

// ---- 测试组件：Counter ----

#[derive(Debug, Clone)]
struct Increment;

impl AppMessage for Increment {
    fn message_name(&self) -> &'static str {
        "increment"
    }
}

#[derive(Debug, Clone)]
struct CountState(u32);

impl Default for CountState {
    fn default() -> Self {
        CountState(0)
    }
}

impl PersistState for CountState {
    fn schema_name() -> &'static str {
        "count_state"
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

struct Counter;

impl WidgetSpec for Counter {
    type State = CountState;
    type Message = Increment;

    fn name(&self) -> &'static str {
        "counter"
    }

    fn view(&self, state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut view = WidgetView::empty();
        view.props = PropValue::Int(state.0 as i64);
        view
    }

    fn update(&self, _msg: Self::Message, state: &mut Self::State, _ctx: &mut UpdateContext) {
        state.0 += 1;
    }

    fn measure(
        &self,
        _state: &Self::State,
        _constraints: BoxConstraints,
        _ctx: &MeasureContext,
    ) -> Size {
        Size::default()
    }

    fn paint(
        &self,
        _state: &Self::State,
        _bounds: Rect,
        _ctx: &mut rgui_core::context::PaintContext,
    ) {
    }

    fn accessibility(
        &self,
        _s: &Self::State,
        _c: &rgui_core::context::AccessContext,
    ) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

// ---- 测试 ----

fn count_from_view(view: &WidgetView<Increment>) -> u32 {
    match view.props {
        PropValue::Int(v) => v as u32,
        _ => panic!("expected Int count in view.props"),
    }
}

#[test]
fn coordinator_dispatches_update_and_view() {
    let mut host = Coordinator::new(Counter, CountState::default());

    // 初始视图：count = 0（host 持有组件 + 状态）
    let v0 = host.current_view(&ViewContext::default());
    assert_eq!(count_from_view(&v0), 0, "初始 count 应为 0");

    // dispatch(Increment) → 内部 update + 产出新视图 → count = 1
    let v1 = host.dispatch(Increment, &mut UpdateContext::default());
    assert_eq!(count_from_view(&v1), 1, "一次 increment 后 count 应为 1");

    // 再次 dispatch → count = 2（状态累积）
    let v2 = host.dispatch(Increment, &mut UpdateContext::default());
    assert_eq!(count_from_view(&v2), 2, "两次 increment 后 count 应为 2");
}

#[test]
fn coordinator_reflects_state_in_view() {
    let mut host = Coordinator::new(Counter, CountState(41));
    // 用注入的初始状态，验证 view 反映状态
    let v = host.current_view(&ViewContext::default());
    assert_eq!(count_from_view(&v), 41, "view 应反映初始状态 41");
    // 不 dispatch，view 仍为初始状态
    assert_eq!(host.state().0, 41);
}

#[test]
fn coordinator_name_and_state_access() {
    let host = Coordinator::new(Counter, CountState(5));
    assert_eq!(host.name(), "counter");
    assert_eq!(host.state().0, 5);
}
