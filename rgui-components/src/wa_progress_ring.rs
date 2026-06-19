/// Translated from Web Awesome wa-progress-ring
/// Original license: MIT
/// Copyright (c) Font Awesome
use ordered_float::OrderedFloat;
use rgui_core::WidgetId;
use rgui_core::a11y::{AccessibilityNode, AccessibilityRole};
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

/// Web Awesome wa-progress-ring 组件状态。
///
/// 环形进度指示器——以圆形填充展示操作进度。
/// 用于空间有限的场景，是进度条的紧凑替代方案。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaProgressRingState {
    /// 当前进度百分比，0 到 100。
    pub value: f64,
    /// 辅助设备自定义标签。
    pub label: String,
}

impl WaProgressRingState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: 0.0,
            label: String::new(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// 环形进度无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaProgressRingMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaProgressRing;

impl WidgetSpec for WaProgressRing {
    type State = WaProgressRingState;
    type Message = WaProgressRingMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaProgressRing"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaProgressRing")
            .prop("value", PropValue::Float(OrderedFloat(state.value)))
            .prop(
                "label",
                PropValue::str(if state.label.is_empty() {
                    "progress"
                } else {
                    state.label.as_str()
                }),
            )
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaProgressRingMessage::NoOp => {},
        }
    }

    /// ProgressRing 尺寸由 Taffy 布局决定（default_layout_for_type 注入 min_size）。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 颜色映射（来自 WA CSS 变量）：
        // --track-color: var(--wa-color-neutral-fill-normal) → 浅灰
        // --indicator-color: var(--wa-color-brand-fill-loud) → 品牌蓝色
        let track_color = Color::new(0.85, 0.85, 0.85, 1.0);
        let indicator_color = Color::new(0.0, 0.42, 0.84, 1.0); // #006BD6
        let text_color = Color::new(0.2, 0.2, 0.2, 1.0); // 深灰文字

        let w: f64 = bounds.size.width;
        let h: f64 = bounds.size.height;
        let min_dim = w.min(h);

        if min_dim < 4.0 {
            return; // 太小，无法绘制
        }

        let cx: f64 = bounds.origin.x + w / 2.0;
        let cy: f64 = bounds.origin.y + h / 2.0;

        // 环形参数
        let outer_radius: f64 = min_dim / 2.0 - 2.0;
        let ring_thickness: f64 = f64::max(3.0, min_dim * 0.08);
        let mid_radius: f64 = outer_radius - ring_thickness / 2.0;

        // 分段数量：确保每个 segment 的大小与 ring_thickness 匹配
        let circumference: f64 = 2.0 * std::f64::consts::PI * mid_radius;
        let n_segments: usize = f64::max(32.0, (circumference / ring_thickness).round()) as usize;
        let n_segments = n_segments.clamp(32, 128);
        let seg_angle: f64 = 2.0 * std::f64::consts::PI / n_segments as f64;
        let seg_size: f64 = ring_thickness;

        // 从顶部开始绘制，对应 WA 的 rotate(-90deg)
        let start_angle: f64 = -std::f64::consts::PI / 2.0;

        // 计算指示器覆盖的段数
        let clamped_value = state.value.clamp(0.0, 100.0);
        let indicator_segments: usize =
            ((clamped_value / 100.0) * n_segments as f64).round() as usize;

        // 绘制环轨（全部段）+ 指示器（前 indicator_segments 段）
        for i in 0..n_segments {
            let angle = start_angle + i as f64 * seg_angle;
            let seg_x = cx + mid_radius * angle.cos() - seg_size / 2.0;
            let seg_y = cy + mid_radius * angle.sin() - seg_size / 2.0;
            let seg_rect = Rect::new(seg_x, seg_y, seg_size, seg_size);

            if i < indicator_segments {
                // 先画轨道色（作为底色），再画指示器色覆盖
                ctx.fill_rect(seg_rect, track_color, 0.0);
                ctx.fill_rect(seg_rect, indicator_color, 0.0);
            } else {
                ctx.fill_rect(seg_rect, track_color, 0.0);
            }
        }

        // 绘制中心标签 —— 显示百分比
        if clamped_value > 0.0 {
            let font_size: f32 = (min_dim * 0.2) as f32;
            let label_text = format!("{:.0}%", clamped_value);
            // 将文本居中于环形内部区域
            let inner_r = mid_radius - ring_thickness;
            let text_area = Rect::new(
                cx - inner_r,
                cy - inner_r * 0.3,
                inner_r * 2.0,
                inner_r * 0.6,
            );
            ctx.draw_text(&label_text, text_area, text_color, font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label_text = if state.label.is_empty() {
            "progress".to_string()
        } else {
            state.label.clone()
        };

        let value_text = format!("{}%", state.value.round() as i32);

        AccessibilityNode::new(
            WidgetId::from_u64(0), // widget_id 由框架填充
            AccessibilityRole::ProgressBar,
            Rect::ZERO,
        )
        .label(label_text)
        .value(value_text)
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
        let state = WaProgressRingState::new();
        assert_eq!(state.value, 0.0);
        assert!(state.label.is_empty());
    }

    #[test]
    fn state_with_value() {
        let state = WaProgressRingState {
            value: 75.0,
            ..WaProgressRingState::new()
        };
        assert_eq!(state.value, 75.0);
    }

    #[test]
    fn state_with_label() {
        let state = WaProgressRingState {
            label: "Uploading files".into(),
            ..WaProgressRingState::new()
        };
        assert_eq!(state.label, "Uploading files");
    }

    #[test]
    fn message_noop() {
        let mut state = WaProgressRingState::new();
        let mut ctx = UpdateContext::default();
        WaProgressRing.update(WaProgressRingMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.value, 0.0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaProgressRingState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaProgressRing.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_zero_value_produces_track_only() {
        let state = WaProgressRingState::new(); // value = 0
        let bounds = Rect::new(0.0, 0.0, 80.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressRing.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        // 所有 segments 都是轨道色（每个 1 个 FillRect），无指示器覆盖
        assert!(fill_count > 0, "应至少有一些 FillRect（轨道段）");
        // 无 DrawText（value=0 不显示标签）
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 0, "value=0 不应有 DrawText");
    }

    #[test]
    fn paint_with_value_produces_ring() {
        let state = WaProgressRingState {
            value: 50.0,
            ..WaProgressRingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 80.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressRing.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert!(fill_count > 0, "应产生 FillRect 操作");
    }

    #[test]
    fn paint_full_value() {
        let state = WaProgressRingState {
            value: 100.0,
            ..WaProgressRingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 80.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressRing.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert!(fill_count > 0, "value=100 应产生 FillRect");
    }

    #[test]
    fn paint_clamps_value_to_100() {
        let state = WaProgressRingState {
            value: 150.0,
            ..WaProgressRingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 80.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressRing.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert!(fill_count > 0);
    }

    #[test]
    fn paint_clamps_value_to_0() {
        let state = WaProgressRingState {
            value: -50.0,
            ..WaProgressRingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 80.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressRing.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // value < 0 → clamp 到 0 → 无指示器、无标签
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 0, "负值 clamp 到 0 → 无标签");
    }

    #[test]
    fn paint_too_small_bounds_returns_early() {
        let state = WaProgressRingState {
            value: 50.0,
            ..WaProgressRingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 3.0, 3.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressRing.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "过小 bounds 应提前返回");
    }

    #[test]
    fn accessibility_default_state() {
        let state = WaProgressRingState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaProgressRing.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("progress"));
        assert_eq!(node.value.as_deref(), Some("0%"));
    }

    #[test]
    fn accessibility_with_custom_label() {
        let state = WaProgressRingState {
            label: "File upload".into(),
            value: 60.0,
            ..WaProgressRingState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaProgressRing.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("File upload"));
        assert_eq!(node.value.as_deref(), Some("60%"));
    }

    #[test]
    fn view_default() {
        let state = WaProgressRingState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaProgressRing.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaProgressRing");
    }

    #[test]
    fn view_with_value_and_label() {
        let state = WaProgressRingState {
            value: 42.0,
            label: "Loading".into(),
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaProgressRing.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaProgressRing");

        if let Some(PropValue::Float(v)) = view.props.get("value") {
            assert_eq!(v.0, 42.0);
        } else {
            panic!("Expected Float prop 'value'");
        }

        if let Some(PropValue::Str(s)) = view.props.get("label") {
            assert_eq!(s.as_ref(), "Loading");
        } else {
            panic!("Expected Str prop 'label'");
        }
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaProgressRing.name(), "rgui_components::WaProgressRing");
    }
}
