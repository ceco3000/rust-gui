/// Translated from Web Awesome wa-progress-bar
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

/// Web Awesome wa-progress-bar 组件状态。
///
/// 进度条——以水平填充展示操作进度。用于文件上传、多步骤流程等可度量进度的任务。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaProgressBarState {
    /// 当前进度百分比，0 到 100。
    pub value: f64,
    /// 为 true 时忽略百分比，隐藏标签，以不确定状态绘制。
    pub indeterminate: bool,
    /// 辅助设备自定义标签。
    pub label: String,
}

impl WaProgressBarState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: 0.0,
            indeterminate: false,
            label: String::new(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// 进度条无交互事件，占位枚举。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaProgressBarMessage {
    #[allow(dead_code)]
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaProgressBar;

impl WidgetSpec for WaProgressBar {
    type State = WaProgressBarState;
    type Message = WaProgressBarMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaProgressBar"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaProgressBar")
            .prop("value", PropValue::Float(OrderedFloat(state.value)))
            .prop("indeterminate", PropValue::Bool(state.indeterminate))
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
            WaProgressBarMessage::NoOp => {},
        }
    }

    /// ProgressBar 是视觉组件，高度固定（track-height），宽度由 Taffy 布局决定。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 颜色映射（来自 WA CSS 变量）：
        // --track-color: var(--wa-color-neutral-fill-normal) → 浅灰
        // --indicator-color: var(--wa-color-brand-fill-loud) → 品牌蓝色
        // --wa-color-brand-on-loud → 白色文字（指示器内标签）
        let track_color = Color::new(0.85, 0.85, 0.85, 1.0);
        let indicator_color = Color::new(0.0, 0.42, 0.84, 1.0); // #006BD6 approx
        let _label_color = Color::WHITE;

        // track-height = 1rem = 16px，由 default_layout_for_type 的 min_size 注入
        // 圆角：pill（全圆角 = 高度/2）
        let border_radius: f32 = if bounds.size.height > 0.0 {
            (bounds.size.height / 2.0) as f32
        } else {
            999.0
        };

        // 绘制轨道（全宽背景）
        ctx.fill_rect(bounds, track_color, border_radius);

        // 计算指示器宽度
        let indicator_width: f64 = if state.indeterminate {
            // 阶段 0：不确定状态用 50% 宽度的静态指示器
            bounds.size.width * 0.5
        } else {
            let clamped_value = state.value.clamp(0.0, 100.0);
            bounds.size.width * (clamped_value / 100.0)
        };

        if indicator_width > 0.0 {
            let indicator_bounds = Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                indicator_width,
                bounds.size.height,
            );
            ctx.fill_rect(indicator_bounds, indicator_color, border_radius);
        }

        // 如果非 indeterminate 且 value > 0，在指示器内绘制百分比文本
        if !state.indeterminate && state.value > 0.0 {
            let font_size: f32 = (bounds.size.height * 0.68) as f32; // ~s font size ratio
            let label_text = format!("{:.0}%", state.value);
            let label_bounds = Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                indicator_width,
                bounds.size.height,
            );
            ctx.draw_text(&label_text, label_bounds, Color::WHITE, font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label_text = if state.label.is_empty() {
            "progress".to_string()
        } else {
            state.label.clone()
        };

        let value_text = if state.indeterminate {
            "indeterminate".to_string()
        } else {
            format!("{}%", state.value.round() as i32)
        };

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
        let state = WaProgressBarState::new();
        assert_eq!(state.value, 0.0);
        assert!(!state.indeterminate);
        assert!(state.label.is_empty());
    }

    #[test]
    fn state_with_value() {
        let state = WaProgressBarState {
            value: 75.0,
            ..WaProgressBarState::new()
        };
        assert_eq!(state.value, 75.0);
    }

    #[test]
    fn state_with_indeterminate() {
        let state = WaProgressBarState {
            indeterminate: true,
            ..WaProgressBarState::new()
        };
        assert!(state.indeterminate);
    }

    #[test]
    fn state_with_label() {
        let state = WaProgressBarState {
            label: "Uploading files".into(),
            ..WaProgressBarState::new()
        };
        assert_eq!(state.label, "Uploading files");
    }

    #[test]
    fn message_noop() {
        let mut state = WaProgressBarState::new();
        let mut ctx = UpdateContext::default();
        WaProgressBar.update(WaProgressBarMessage::NoOp, &mut state, &mut ctx);
        // 无操作，状态不变
        assert_eq!(state.value, 0.0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaProgressBarState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaProgressBar.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_zero_value_produces_track_only() {
        let state = WaProgressBarState::new(); // value = 0
        let bounds = Rect::new(0.0, 0.0, 300.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressBar.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 应有 1 个 FillRect（仅轨道，value=0 不画指示器）
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 1, "value=0 应只有一个 FillRect（轨道）");
    }

    #[test]
    fn paint_with_value_produces_track_and_indicator() {
        let state = WaProgressBarState {
            value: 50.0,
            ..WaProgressBarState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressBar.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 应有 2 个 FillRect（轨道 + 指示器） + 可能的 DrawText
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert!(fill_count >= 2, "value=50 应有至少 2 个 FillRect");
    }

    #[test]
    fn paint_full_value_indicator_fills_entire_width() {
        let state = WaProgressBarState {
            value: 100.0,
            ..WaProgressBarState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressBar.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert!(fill_count >= 2, "value=100 应有至少 2 个 FillRect");
    }

    #[test]
    fn paint_indeterminate_produces_indicator() {
        let state = WaProgressBarState {
            indeterminate: true,
            ..WaProgressBarState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressBar.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert!(fill_count >= 2, "indeterminate 应有至少 2 个 FillRect");
    }

    #[test]
    fn paint_value_at_zero_no_indicator() {
        let state = WaProgressBarState::new(); // value = 0
        let bounds = Rect::new(0.0, 0.0, 300.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressBar.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // value=0 且非 indeterminate → 仅有轨道
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 1);
    }

    #[test]
    fn paint_clamps_value_to_100() {
        let state = WaProgressBarState {
            value: 150.0,
            ..WaProgressBarState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressBar.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert!(fill_count >= 2, "clamped value 应产生指示器");
    }

    #[test]
    fn paint_clamps_value_to_0() {
        let state = WaProgressBarState {
            value: -50.0,
            ..WaProgressBarState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 16.0);
        let mut ctx = PaintContext::new(bounds);
        WaProgressBar.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let fill_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 1, "负值 clamp 到 0 → 仅轨道");
    }

    #[test]
    fn accessibility_default_state() {
        let state = WaProgressBarState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaProgressBar.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("progress"));
        assert_eq!(node.value.as_deref(), Some("0%"));
    }

    #[test]
    fn accessibility_with_custom_label() {
        let state = WaProgressBarState {
            label: "File upload".into(),
            value: 60.0,
            ..WaProgressBarState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaProgressBar.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("File upload"));
        assert_eq!(node.value.as_deref(), Some("60%"));
    }

    #[test]
    fn accessibility_indeterminate() {
        let state = WaProgressBarState {
            indeterminate: true,
            ..WaProgressBarState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaProgressBar.accessibility(&state, &access_ctx);
        assert_eq!(node.value.as_deref(), Some("indeterminate"));
    }

    #[test]
    fn view_default() {
        let state = WaProgressBarState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaProgressBar.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaProgressBar");
    }

    #[test]
    fn view_with_value_and_indeterminate() {
        let state = WaProgressBarState {
            value: 42.0,
            indeterminate: true,
            label: "Loading".into(),
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaProgressBar.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaProgressBar");

        // 验证 value prop
        if let Some(PropValue::Float(v)) = view.props.get("value") {
            assert_eq!(v.0, 42.0);
        } else {
            panic!("Expected Float prop 'value'");
        }

        // 验证 indeterminate prop
        if let Some(PropValue::Bool(b)) = view.props.get("indeterminate") {
            assert!(*b);
        } else {
            panic!("Expected Bool prop 'indeterminate'");
        }

        // 验证 label prop
        if let Some(PropValue::Str(s)) = view.props.get("label") {
            assert_eq!(s.as_ref(), "Loading");
        } else {
            panic!("Expected Str prop 'label'");
        }
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaProgressBar.name(), "rgui_components::WaProgressBar");
    }
}
