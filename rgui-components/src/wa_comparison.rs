/// Translated from Web Awesome wa-comparison
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

/// Web Awesome wa-comparison 组件状态。
///
/// ImageComparer 用拖拽分隔条展示两张相似图片的视觉差异，
/// 常用于 before/after 对比、设计修订、并排预览等场景。
/// Phase 0：仅支持静态分隔条位置，暂不支持拖拽手势、键盘导航、clipPath 裁剪。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaComparisonState {
    /// 分隔条当前位置（0-100 百分比，默认 50）
    pub position: f64,
}

impl WaComparisonState {
    #[must_use]
    pub fn new() -> Self {
        Self { position: 50.0 }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaComparisonMessage {
    /// 分隔条位置改变时发出（wa-change 事件映射）
    Change,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaComparison;

impl WidgetSpec for WaComparison {
    type State = WaComparisonState;
    type Message = WaComparisonMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaComparison"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaComparison").prop(
            "position",
            PropValue::Float(ordered_float::OrderedFloat(state.position)),
        )
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaComparisonMessage::Change => {
                // Phase 0: change 事件通过外部状态更新处理
            },
        }
    }

    /// Comparison 是容器，尺寸由 Taffy 根据子节点和约束计算
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    /// Phase 0 渲染：垂直分隔线 + 圆形拖拽手柄。
    ///
    /// 分隔线位于 position% 处，手柄在分隔线中心。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let divider_width: f64 = 3.0;
        let handle_size: f64 = 28.0;

        let x = bounds.origin.x;
        let y = bounds.origin.y;
        let w = bounds.size.width;
        let h = bounds.size.height;

        if w <= 0.0 || h <= 0.0 {
            return;
        }

        // 分隔线颜色：中性灰
        let divider_color = Color::new(0.6, 0.6, 0.6, 0.9);

        // 垂直分隔线位于 position% 处
        let divider_x = x + (state.position / 100.0) * w - divider_width / 2.0;
        let clamped_x = divider_x.max(x).min(x + w - divider_width);
        ctx.fill_rect(
            Rect::new(clamped_x, y, divider_width, h),
            divider_color,
            0.0,
        );

        // 手柄：圆形，位于分隔线垂直居中位置
        let handle_radius: f32 = (handle_size / 2.0) as f32;
        let handle_x = clamped_x + divider_width / 2.0 - handle_size / 2.0;
        let handle_y = y + (h - handle_size) / 2.0;
        let handle_rect = Rect::new(handle_x, handle_y, handle_size, handle_size);

        // 手柄背景（白色圆）
        let handle_bg = Color::new(1.0, 1.0, 1.0, 1.0);
        ctx.fill_rect(handle_rect, handle_bg, handle_radius);

        // 手柄边框（与分隔线同色）
        let handle_border = divider_color;
        let border_width: f64 = 1.5;
        // 用略小的填充矩形模拟边框（外白圆 + 内灰圆）
        let inner_inset = border_width;
        let inner_rect = Rect::new(
            handle_x + inner_inset,
            handle_y + inner_inset,
            handle_size - 2.0 * inner_inset,
            handle_size - 2.0 * inner_inset,
        );
        ctx.fill_rect(
            inner_rect,
            handle_border,
            handle_radius - border_width as f32,
        );

        // 手柄内的竖线图标（模拟 grip-vertical）
        let grip_color = Color::new(1.0, 1.0, 1.0, 1.0);
        let grip_bar_w: f64 = 2.0;
        let grip_bar_h: f64 = handle_size * 0.35;
        let grip_center_x = handle_x + handle_size / 2.0;
        let grip_center_y = handle_y + handle_size / 2.0;

        // 左竖条
        let left_bar_x = grip_center_x - grip_bar_w - 2.0;
        ctx.fill_rect(
            Rect::new(
                left_bar_x,
                grip_center_y - grip_bar_h / 2.0,
                grip_bar_w,
                grip_bar_h,
            ),
            grip_color,
            0.5,
        );
        // 右竖条
        let right_bar_x = grip_center_x + 2.0;
        ctx.fill_rect(
            Rect::new(
                right_bar_x,
                grip_center_y - grip_bar_h / 2.0,
                grip_bar_w,
                grip_bar_h,
            ),
            grip_color,
            0.5,
        );
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label = format!("Image comparison — {}%", state.position);
        AccessibilityNode::none().label(label)
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
        let state = WaComparisonState::new();
        assert!(
            (state.position - 50.0).abs() < 0.01,
            "默认 position 应为 50"
        );
    }

    #[test]
    fn state_with_position() {
        let state = WaComparisonState { position: 75.0 };
        assert!((state.position - 75.0).abs() < 0.01);
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaComparison.name(), "rgui_components::WaComparison");
    }

    #[test]
    fn measure_returns_zero_delegating_to_layout() {
        let state = WaComparisonState::new();
        let size = WaComparison.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Comparison 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_produces_divider_and_handle() {
        let state = WaComparisonState::new();
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaComparison.paint(&state, bounds, &mut ctx);
        // 分隔线 + 手柄背景 + 手柄内框 + 2 grip bars = 5 fills
        assert_eq!(
            ctx.op_count(),
            5,
            "应产生 5 个 fill_rect 操作（分隔线+手柄背景+内框+2 grip）"
        );
    }

    #[test]
    fn paint_position_25_left_side() {
        let state = WaComparisonState { position: 25.0 };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaComparison.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 5);
    }

    #[test]
    fn paint_position_75_right_side() {
        let state = WaComparisonState { position: 75.0 };
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut ctx = PaintContext::new(bounds);
        WaComparison.paint(&state, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 5);
    }

    #[test]
    fn paint_zero_bounds_no_ops() {
        let state = WaComparisonState::new();
        let bounds = Rect::ZERO;
        let mut ctx = PaintContext::new(bounds);
        WaComparison.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "零尺寸不应产生绘制操作");
    }

    #[test]
    fn view_has_position_prop() {
        let state = WaComparisonState::new();
        let v = WaComparison.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("position"));
    }

    #[test]
    fn view_default_position_50() {
        let state = WaComparisonState::new();
        let v = WaComparison.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Float(f) => assert!((f.0 - 50.0).abs() < 0.01),
            _ => panic!("expected Float prop"),
        }
    }

    #[test]
    fn view_position_30() {
        let state = WaComparisonState { position: 30.0 };
        let v = WaComparison.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("position").unwrap();
        match val {
            PropValue::Float(f) => assert!((f.0 - 30.0).abs() < 0.01),
            _ => panic!("expected Float prop"),
        }
    }

    #[test]
    fn update_change_does_not_panic() {
        let mut state = WaComparisonState::new();
        WaComparison.update(
            WaComparisonMessage::Change,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaComparisonMessage::Change.message_name(), "change");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaComparisonState::schema_name(), "WaComparisonState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaComparisonState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaComparisonState>());
    }

    #[test]
    fn accessibility_label_contains_position() {
        let state = WaComparisonState { position: 42.0 };
        let node = WaComparison.accessibility(&state, &AccessContext::new(Rect::ZERO));
        let label = node.label.as_deref().unwrap();
        assert!(
            label.contains("42%"),
            "无障碍标签应包含位置百分比，实际: {label}"
        );
    }

    #[test]
    fn accessibility_default_label() {
        let state = WaComparisonState::new();
        let node = WaComparison.accessibility(&state, &AccessContext::new(Rect::ZERO));
        let label = node.label.as_deref().unwrap();
        assert!(label.contains("50%"));
    }
}
