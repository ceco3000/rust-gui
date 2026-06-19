/// Translated from Web Awesome wa-skeleton
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-skeleton 组件状态。
///
/// 骨架屏——在内容加载完成前显示占位形状。
/// 当前阶段 0：静态渲染，所有 effect 值产生相同外观。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaSkeletonState {
    /// 视觉效果：pulse | sheen | none（阶段 0 均为静态）
    pub effect: String,
}

impl WaSkeletonState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            effect: "none".into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// 骨架屏无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaSkeletonMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaSkeleton;

impl WidgetSpec for WaSkeleton {
    type State = WaSkeletonState;
    type Message = WaSkeletonMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaSkeleton"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaSkeleton")
            .prop("effect", PropValue::str(state.effect.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaSkeletonMessage::NoOp => {},
        }
    }

    /// Skeleton 是装饰占位组件，尺寸由 Taffy 布局决定。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 颜色：neutral-fill-normal（浅色中性灰背景）
        let fill_color = Color::new(0.85, 0.85, 0.85, 1.0);

        // ⚠️ `effect` 在阶段 0 不区分视觉效果（框架暂无动画支持）
        // pulse 和 sheen 均回退为静态填充矩形
        let _ = state.effect.as_str();

        // 圆角：pill（全圆角，模拟 WA --wa-border-radius-pill）
        let border_radius: f32 = if bounds.size.height > 0.0 {
            (bounds.size.height / 2.0) as f32
        } else {
            999.0
        };

        ctx.fill_rect(bounds, fill_color, border_radius);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let _ = state;
        AccessibilityNode::none()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::geometry::{BoxConstraints, Rect, Size};

    #[test]
    fn default_state() {
        let state = WaSkeletonState::new();
        assert_eq!(state.effect, "none");
    }

    #[test]
    fn state_with_custom_effect() {
        let state = WaSkeletonState {
            effect: "pulse".into(),
            ..WaSkeletonState::new()
        };
        assert_eq!(state.effect, "pulse");
    }

    #[test]
    fn message_noop() {
        let mut state = WaSkeletonState::new();
        let mut ctx = UpdateContext::default();
        WaSkeleton.update(WaSkeletonMessage::NoOp, &mut state, &mut ctx);
        // 无操作，状态不变
        assert_eq!(state.effect, "none");
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaSkeletonState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaSkeleton.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_produces_fill_rect() {
        let state = WaSkeletonState::new();
        let bounds = Rect::new(0.0, 0.0, 200.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaSkeleton.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(!ops.is_empty(), "Skeleton 应产生绘制操作");
        // 应该有一个 FillRect
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 1, "应有恰好 1 个 FillRect");
    }

    #[test]
    fn paint_all_effect_variants_produce_same_result() {
        for effect in &["none", "pulse", "sheen"] {
            let state = WaSkeletonState {
                effect: (*effect).into(),
                ..WaSkeletonState::new()
            };
            let bounds = Rect::new(0.0, 0.0, 100.0, 16.0);
            let mut ctx = PaintContext::new(bounds);
            WaSkeleton.paint(&state, bounds, &mut ctx);
            let ops = ctx.into_operations();
            assert!(!ops.is_empty(), "effect={effect} 应产生绘制操作");
        }
    }

    #[test]
    fn accessibility_returns_none() {
        let state = WaSkeletonState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaSkeleton.accessibility(&state, &access_ctx);
        // 骨架屏对无障碍无意义，返回 None
        assert!(node.label.is_none());
    }

    #[test]
    fn view_contains_effect_prop() {
        let state = WaSkeletonState {
            effect: "pulse".into(),
            ..WaSkeletonState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaSkeleton.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaSkeleton");
        let effect = match view.props.get("effect") {
            Some(PropValue::Str(s)) => Some(s.as_ref()),
            _ => None,
        };
        assert_eq!(effect, Some("pulse"));
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaSkeleton.name(), "rgui_components::WaSkeleton");
    }
}
