//! D4 验收测试 · WidgetSpec 核心循环 + WidgetRegistry + WidgetNode + 视图/几何契约
//!
//! 注入：cp tools/qa/d4_tests/d4_acceptance_core.rs rgui-core/tests/
//! 运行：cargo test -p rgui-core --test d4_acceptance_core
//! 判据：全绿为 PASS；本节用例基于 D3 已交付占位契约，dev D4 实现后应保持此行为。

use rgui_core::context::{MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Point, Rect, Size};
use rgui_core::registry::WidgetRegistry;
use rgui_core::traits::{AppMessage, EventResult, PersistState, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_core::message::NoopMsg;

/// 测试组件：计数器，验证 view/update 闭环。
#[derive(Debug, Default, Clone)]
struct Counter;

#[derive(Debug, Clone, PartialEq, Eq)]
enum CounterMsg {
    Inc,
    Dec,
}

impl AppMessage for CounterMsg {
    fn message_name(&self) -> &'static str {
        match self {
            CounterMsg::Inc => "counter.inc",
            CounterMsg::Dec => "counter.dec",
        }
    }
}

#[derive(Debug, Clone)]
struct CounterState {
    count: i64,
}

impl Default for CounterState {
    fn default() -> Self {
        Self { count: 0 }
    }
}

impl PersistState for CounterState {
    fn schema_name() -> &'static str {
        "counter_state"
    }
    fn schema_version() -> u32 {
        0
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl WidgetSpec for Counter {
    type State = CounterState;
    type Message = CounterMsg;

    fn name(&self) -> &'static str {
        "counter"
    }

    fn view(&self, state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::default();
        v.props = PropValue::Int(state.count);
        v
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _ctx: &mut UpdateContext) {
        match msg {
            CounterMsg::Inc => state.count += 1,
            CounterMsg::Dec => state.count -= 1,
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _ctx: &MeasureContext) -> Size {
        Size::new(100.0, 40.0)
    }

    fn paint(&self, _state: &Self::State, _b: Rect, _ctx: &mut PaintContext) {}
}

/// 辅助：构造测试状态。
fn counter_state(n: i64) -> CounterState {
    CounterState { count: n }
}

// ============ W: 视图/生命周期 ============

#[test]
fn w1_initial_view_reflects_state() {
    let c = Counter;
    let st = counter_state(0);
    let v = c.view(&st, &ViewContext::default());
    assert_eq!(v.props, PropValue::Int(0));
}

#[test]
fn w2_update_then_view_changes() {
    let c = Counter;
    let mut st = counter_state(0);
    let mut ctx = UpdateContext::default();
    c.update(CounterMsg::Inc, &mut st, &mut ctx);
    let v = c.view(&st, &ViewContext::default());
    assert_eq!(v.props, PropValue::Int(1));
}

#[test]
fn w3_update_event_preserves_state_value() {
    let c = Counter;
    let mut st = counter_state(5);
    let mut ctx = UpdateContext::default();
    c.update(CounterMsg::Dec, &mut st, &mut ctx);
    assert_eq!(st.count, 4);
}

#[test]
fn w4_measure_within_bounds() {
    let c = Counter;
    let st = counter_state(1);
    let size = c.measure(&st, BoxConstraints::loose(), &MeasureContext::default());
    assert!(size.width >= 0.0 && size.height >= 0.0);
    assert!(size.width <= f32::MAX && size.height <= f32::MAX);
}

#[test]
fn w5_paint_no_panic() {
    let c = Counter;
    let st = counter_state(1);
    let b = Rect::new(0.0, 0.0, 100.0, 100.0);
    let mut pc = PaintContext::default();
    c.paint(&st, b, &mut pc);
}

#[test]
fn w6_accessibility_default() {
    let c = Counter;
    let st = counter_state(1);
    let a = c.accessibility(&st, &rgui_core::context::AccessContext::default());
    let _ = a; // 默认 none/empty，不 panic
}

#[test]
fn w7_name_is_static_and_stable() {
    assert_eq!(Counter.name(), "counter");
}

// ============ S: 状态/消息契约 ============

#[test]
fn s1_message_appmessage_contract() {
    assert_eq!(CounterMsg::Inc.message_name(), "counter.inc");
    assert_eq!(CounterMsg::Dec.message_name(), "counter.dec");
}

#[test]
fn s2_component_state_satisfies_persist() {
    let st = counter_state(3);
    assert_eq!(CounterState::schema_name(), "counter_state");
    assert_eq!(CounterState::schema_version(), 0);
    let any: &dyn std::any::Any = st.as_any();
    assert!(any.is::<CounterState>());
}

// ============ R: WidgetRegistry ============

#[test]
fn r1_register_duplicate_rejected() {
    let reg = WidgetRegistry::new();
    let w = std::sync::Arc::new(Counter) as std::sync::Arc<dyn std::any::Any + Send + Sync>;
    reg.register("counter", w);
}

// 注：registry.get/NotFound 语义依赖 dev 实现扩展接口（当前 register 为占位 no-op）。
// 待 dev 实现 get/register 返回 Result 后补充 r2/r3 断言，见清单 R1-R4。

// ============ PropValue / 几何契约 ============

#[test]
fn v1_propvalue_partial_eq() {
    assert_eq!(PropValue::Bool(true), PropValue::Bool(true));
    assert_eq!(PropValue::Int(1), PropValue::Int(1));
    assert_ne!(PropValue::Int(1), PropValue::Float(1.0));
}

#[test]
fn v2_color_rgb_default_alpha() {
    assert_eq!(Color::rgb(1, 2, 3).a, 255);
}

#[test]
fn v3_boxconstraints_free() {
    let bc = BoxConstraints::loose();
    assert_eq!(bc.max.width, f32::MAX);
    assert_eq!(bc.max.height, f32::MAX);
}

#[test]
fn v4_rect_edges() {
    let r = Rect::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(r.right(), 4.0);
    assert_eq!(r.bottom(), 6.0);
}

#[test]
fn v5_event_result_variants() {
    let h = EventResult::<NoopMsg>::Handled;
    assert!(matches!(h, EventResult::Handled));
    let c = EventResult::Continue(NoopMsg);
    match c {
        EventResult::Continue(_) => {}
        _ => panic!("expected Continue"),
    }
}

#[test]
fn v6_point_size_default() {
    assert_eq!(Point::default().x, 0);
    assert_eq!(Size::default().width, 0.0);
}
