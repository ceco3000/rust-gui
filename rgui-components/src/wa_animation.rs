/// Translated from Web Awesome wa-animation
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-animation 组件状态。
///
/// 动画控制器——用声明式属性驱动子元素动画。
/// Phase 0：属性存储 + 槽位透传，无实际 CSS 动画引擎。
/// Phase 1（后续）：集成 Animated<T> 实现关键帧动画。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaAnimationState {
    /// 内置动画名称（如 "bounce", "fadeIn"），或 "none" 表示无动画
    pub name: String,
    /// 是否播放动画。动画完成或取消时自动设为 false
    pub play: bool,
    /// 动画开始前的延迟（毫秒）
    pub delay: f64,
    /// 播放方向：normal | reverse | alternate | alternate-reverse
    pub direction: String,
    /// 单次迭代持续时间（毫秒）
    pub duration: f64,
    /// 缓动函数：linear | ease | ease-in | ease-out | ease-in-out | cubic-bezier(...)
    pub easing: String,
    /// 动画活动期结束后的延迟（毫秒）
    pub end_delay: f64,
    /// 动画执行前后如何应用样式：auto | none | forwards | backwards | both
    pub fill: String,
    /// 迭代次数。默认 Infinity 表示循环播放
    pub iterations: f64,
    /// 迭代起始偏移，通常 0（起始）到 1（结束）
    pub iteration_start: f64,
    /// 播放速率。1=正常速度，2=双倍速，负值=反向
    pub playback_rate: f64,
    /// 内部标记：是否已触发过 wa-start 事件
    #[serde(skip)]
    pub has_started: bool,
}

impl WaAnimationState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "none".into(),
            play: false,
            delay: 0.0,
            direction: "normal".into(),
            duration: 1000.0,
            easing: "linear".into(),
            end_delay: 0.0,
            fill: "auto".into(),
            iterations: f64::INFINITY,
            iteration_start: 0.0,
            playback_rate: 1.0,
            has_started: false,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// WaAnimation 事件消息。
///
/// Phase 0：消息变体仅用于 API 契约声明，update() 中均为空处理。
/// 实际事件触发需框架动画引擎（Phase 1）。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaAnimationMessage {
    /// 动画被取消时触发（wa-cancel）
    #[allow(dead_code)]
    Cancel,
    /// 动画正常完成时触发（wa-finish）
    #[allow(dead_code)]
    Finish,
    /// 动画开始/重新开始时触发（wa-start）
    #[allow(dead_code)]
    Start,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaAnimation;

impl WidgetSpec for WaAnimation {
    type State = WaAnimationState;
    type Message = WaAnimationMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaAnimation"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaAnimation")
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("play", PropValue::Bool(state.play))
            .prop(
                "delay",
                PropValue::Float(ordered_float::OrderedFloat(state.delay)),
            )
            .prop("direction", PropValue::str(state.direction.as_str()))
            .prop(
                "duration",
                PropValue::Float(ordered_float::OrderedFloat(state.duration)),
            )
            .prop("easing", PropValue::str(state.easing.as_str()))
            .prop(
                "end_delay",
                PropValue::Float(ordered_float::OrderedFloat(state.end_delay)),
            )
            .prop("fill", PropValue::str(state.fill.as_str()))
            .prop(
                "iterations",
                PropValue::Float(ordered_float::OrderedFloat(state.iterations)),
            )
            .prop(
                "iteration_start",
                PropValue::Float(ordered_float::OrderedFloat(state.iteration_start)),
            )
            .prop(
                "playback_rate",
                PropValue::Float(ordered_float::OrderedFloat(state.playback_rate)),
            )
        // has_started 是内部状态，不作为 prop 暴露
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaAnimationMessage::Cancel | WaAnimationMessage::Finish | WaAnimationMessage::Start => {
                // Phase 0：事件消息无实际操作。Phase 1 由动画引擎触发。
            },
        }
    }

    /// WaAnimation 是容器包装器，尺寸由 Taffy 根据子节点计算。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    /// WaAnimation 无自身视觉绘制——子节点在槽位中渲染。
    fn paint(&self, _state: &Self::State, _bounds: Rect, _ctx: &mut PaintContext) {
        // 动画效果由 Phase 1 的 Animated<T> 在渲染管线中应用
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
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

    #[test]
    fn default_state() {
        let state = WaAnimationState::new();
        assert_eq!(state.name, "none");
        assert!(!state.play);
        assert_eq!(state.delay, 0.0);
        assert_eq!(state.direction, "normal");
        assert_eq!(state.duration, 1000.0);
        assert_eq!(state.easing, "linear");
        assert_eq!(state.end_delay, 0.0);
        assert_eq!(state.fill, "auto");
        assert!(state.iterations.is_infinite());
        assert_eq!(state.iteration_start, 0.0);
        assert_eq!(state.playback_rate, 1.0);
        assert!(!state.has_started);
    }

    #[test]
    fn state_with_custom_name() {
        let state = WaAnimationState {
            name: "bounce".into(),
            ..WaAnimationState::new()
        };
        assert_eq!(state.name, "bounce");
    }

    #[test]
    fn state_with_play_true() {
        let state = WaAnimationState {
            play: true,
            ..WaAnimationState::new()
        };
        assert!(state.play);
    }

    #[test]
    fn state_with_custom_duration() {
        let state = WaAnimationState {
            duration: 500.0,
            ..WaAnimationState::new()
        };
        assert_eq!(state.duration, 500.0);
    }

    #[test]
    fn state_with_custom_easing() {
        let state = WaAnimationState {
            easing: "ease-in-out".into(),
            ..WaAnimationState::new()
        };
        assert_eq!(state.easing, "ease-in-out");
    }

    #[test]
    fn state_with_custom_direction() {
        let state = WaAnimationState {
            direction: "alternate".into(),
            ..WaAnimationState::new()
        };
        assert_eq!(state.direction, "alternate");
    }

    #[test]
    fn state_with_finite_iterations() {
        let state = WaAnimationState {
            iterations: 3.0,
            ..WaAnimationState::new()
        };
        assert!(!state.iterations.is_infinite());
        assert_eq!(state.iterations, 3.0);
    }

    #[test]
    fn state_with_custom_playback_rate() {
        let state = WaAnimationState {
            playback_rate: 2.0,
            ..WaAnimationState::new()
        };
        assert_eq!(state.playback_rate, 2.0);
    }

    #[test]
    fn state_custom_end_delay() {
        let state = WaAnimationState {
            end_delay: 200.0,
            ..WaAnimationState::new()
        };
        assert_eq!(state.end_delay, 200.0);
    }

    #[test]
    fn state_custom_fill() {
        let state = WaAnimationState {
            fill: "forwards".into(),
            ..WaAnimationState::new()
        };
        assert_eq!(state.fill, "forwards");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaAnimationMessage::Start.message_name(), "start");
        assert_eq!(WaAnimationMessage::Finish.message_name(), "finish");
        assert_eq!(WaAnimationMessage::Cancel.message_name(), "cancel");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaAnimationState::schema_name(), "WaAnimationState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaAnimationState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaAnimationState>());
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaAnimation.name(), "rgui_components::WaAnimation");
    }

    #[test]
    fn view_has_name_prop() {
        let state = WaAnimationState {
            name: "fadeIn".into(),
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaAnimation");
        let name = match view.props.get("name") {
            Some(PropValue::Str(s)) => Some(s.as_ref()),
            _ => None,
        };
        assert_eq!(name, Some("fadeIn"));
    }

    #[test]
    fn view_has_play_prop() {
        let state = WaAnimationState {
            play: true,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        assert!(matches!(
            view.props.get("play"),
            Some(PropValue::Bool(true))
        ));
    }

    #[test]
    fn view_play_false_by_default() {
        let state = WaAnimationState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        assert!(matches!(
            view.props.get("play"),
            Some(PropValue::Bool(false))
        ));
    }

    #[test]
    fn view_has_delay_prop() {
        let state = WaAnimationState {
            delay: 300.0,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let delay = match view.props.get("delay") {
            Some(PropValue::Float(f)) => Some(f.0),
            _ => None,
        };
        assert_eq!(delay, Some(300.0));
    }

    #[test]
    fn view_has_duration_prop() {
        let state = WaAnimationState {
            duration: 500.0,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let duration = match view.props.get("duration") {
            Some(PropValue::Float(f)) => Some(f.0),
            _ => None,
        };
        assert_eq!(duration, Some(500.0));
    }

    #[test]
    fn view_has_easing_prop() {
        let state = WaAnimationState {
            easing: "ease-in".into(),
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let easing = match view.props.get("easing") {
            Some(PropValue::Str(s)) => Some(s.as_ref()),
            _ => None,
        };
        assert_eq!(easing, Some("ease-in"));
    }

    #[test]
    fn view_has_direction_prop() {
        let state = WaAnimationState {
            direction: "reverse".into(),
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let direction = match view.props.get("direction") {
            Some(PropValue::Str(s)) => Some(s.as_ref()),
            _ => None,
        };
        assert_eq!(direction, Some("reverse"));
    }

    #[test]
    fn view_has_end_delay_prop() {
        let state = WaAnimationState {
            end_delay: 150.0,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let end_delay = match view.props.get("end_delay") {
            Some(PropValue::Float(f)) => Some(f.0),
            _ => None,
        };
        assert_eq!(end_delay, Some(150.0));
    }

    #[test]
    fn view_has_fill_prop() {
        let state = WaAnimationState {
            fill: "both".into(),
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let fill = match view.props.get("fill") {
            Some(PropValue::Str(s)) => Some(s.as_ref()),
            _ => None,
        };
        assert_eq!(fill, Some("both"));
    }

    #[test]
    fn view_has_iterations_prop() {
        let state = WaAnimationState {
            iterations: 5.0,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let iterations = match view.props.get("iterations") {
            Some(PropValue::Float(f)) => Some(f.0),
            _ => None,
        };
        assert_eq!(iterations, Some(5.0));
    }

    #[test]
    fn view_has_iteration_start_prop() {
        let state = WaAnimationState {
            iteration_start: 0.5,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let iteration_start = match view.props.get("iteration_start") {
            Some(PropValue::Float(f)) => Some(f.0),
            _ => None,
        };
        assert_eq!(iteration_start, Some(0.5));
    }

    #[test]
    fn view_has_playback_rate_prop() {
        let state = WaAnimationState {
            playback_rate: 2.0,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        let playback_rate = match view.props.get("playback_rate") {
            Some(PropValue::Float(f)) => Some(f.0),
            _ => None,
        };
        assert_eq!(playback_rate, Some(2.0));
    }

    #[test]
    fn update_cancel_does_not_panic() {
        let mut state = WaAnimationState::new();
        WaAnimation.update(
            WaAnimationMessage::Cancel,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_finish_does_not_panic() {
        let mut state = WaAnimationState::new();
        WaAnimation.update(
            WaAnimationMessage::Finish,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_start_does_not_panic() {
        let mut state = WaAnimationState::new();
        WaAnimation.update(
            WaAnimationMessage::Start,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaAnimationState::new();
        let constraints = BoxConstraints::new(0.0, 800.0, 0.0, 600.0);
        let size = WaAnimation.measure(&state, constraints, &MeasureContext::default());
        assert_eq!(size, Size::ZERO, "WaAnimation 委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_produces_no_ops() {
        let state = WaAnimationState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        WaAnimation.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "WaAnimation 无自身视觉绘制");
    }

    #[test]
    fn accessibility_returns_none() {
        let state = WaAnimationState::new();
        let node = WaAnimation.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert!(node.label.is_none());
    }

    #[test]
    fn has_started_not_exposed_as_prop() {
        let state = WaAnimationState {
            has_started: true,
            ..WaAnimationState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        // has_started 是内部状态，不应作为 prop 暴露
        assert!(!view.props.contains_key("has_started"));
    }

    #[test]
    fn view_produces_11_props() {
        let state = WaAnimationState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaAnimation.view(&state, &ctx);
        // 11 个公共属性：name, play, delay, direction, duration, easing,
        // end_delay, fill, iterations, iteration_start, playback_rate
        assert_eq!(view.props.len(), 11);
    }
}
