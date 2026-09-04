//! 最小可运行示例：核心循环（view→update→view）+ diff + snapshot 状态管理闭环。
//!
//! 演示一个计数器组件（Counter）经 Coordinator 驱动：状态变化 → 视图更新，
//! 并用 diff 展示 0→1→2 的视图差分、Snapshot 展示可序列化快照。
//!
//! 运行：cargo run -p rgui --example demo

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::coordinator::Coordinator;
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::state::{diff, Snapshotter};
use rgui_core::traits::{AppMessage, PersistState, WidgetSpec};
use rgui_core::view::{PropValue, WidgetView};
use std::any::Any;

#[derive(Debug, Clone, Copy)]
struct Increment;
impl AppMessage for Increment {
    fn message_name(&self) -> &'static str {
        "increment"
    }
}

#[derive(Debug, Clone)]
struct CounterState(u32);
impl Default for CounterState {
    fn default() -> Self {
        CounterState(0)
    }
}
impl PersistState for CounterState {
    fn schema_name() -> &'static str {
        "counter_state"
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
    type State = CounterState;
    type Message = Increment;
    fn name(&self) -> &'static str {
        "counter"
    }
    fn view(&self, state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::empty();
        v.props = PropValue::Int(state.0 as i64);
        v
    }
    fn update(&self, _msg: Self::Message, state: &mut Self::State, _ctx: &mut UpdateContext) {
        state.0 += 1;
    }
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _ctx: &MeasureContext) -> Size {
        Size::new(100.0, 40.0)
    }
    fn paint(&self, _state: &Self::State, _bounds: Rect, _ctx: &mut PaintContext) {}
    fn accessibility(
        &self,
        _s: &Self::State,
        _c: &rgui_core::context::AccessContext,
    ) -> AccessibilityNode {
        AccessibilityNode::none()
    }
}

fn main() {
    println!("== rgui 最小核心循环演示 ==");

    let mut host = Coordinator::new(Counter, CounterState::default());

    // 视图 0
    let v0 = host.current_view(&ViewContext::default());
    println!("[view 0] props = {:?}", v0.props);

    // update → view 1
    let v1 = host.dispatch(Increment, &mut UpdateContext::default());
    println!("[view 1] props = {:?}", v1.props);

    // diff(0→1)
    let patches = diff(&v0, &v1);
    println!("[diff 0→1] {:?} patches", patches.len());

    // update → view 2
    let v2 = host.dispatch(Increment, &mut UpdateContext::default());
    println!("[view 2] props = {:?}", v2.props);

    // snapshot（状态管理可序列化快照）
    let snapshotter = Snapshotter::new().with_schema("demo_counter", 1);
    let mut snap = snapshotter.snapshot();
    snap.insert_state(rgui_core::id::WidgetId::new(1), v2.props.clone());
    println!(
        "[snapshot] schema={} v{} instances={}",
        snap.schema_name,
        snap.schema_version,
        snap.instances.len()
    );

    println!("== 核心循环闭环完成：0 → {} ==", host.state().0);
}
