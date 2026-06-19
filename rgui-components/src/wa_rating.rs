/// Translated from Web Awesome wa-rating
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

/// Web Awesome wa-rating 组件状态。
///
/// Rating（评分）显示一组可选符号（通常为星星），用于快速反馈或展示平均评分。
///
/// 简化项：
/// - getSymbol 回调跳过（不可移植的 JS 函数，改用 Unicode 星星字符）
/// - hoverValue/isHovering 悬停交互跳过（Phase 0 无指针跟踪）
/// - 键盘导航（ArrowLeft/Right/Home/End）跳过
/// - FormField trait impl 跳过（trait 尚未提交）
/// - precision 部分填充简化——Phase 0 用四舍五入
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaRatingState {
    /// 当前评分值。
    pub value: f64,
    /// 默认值（表单重置时使用）。
    pub default_value: f64,
    /// 最高评分（星星数量）。
    pub max: f64,
    /// 步长精度（例如 0.5 允许半星评分）。
    pub precision: f64,
    /// 只读模式。
    pub readonly: bool,
    /// 禁用状态。
    pub disabled: bool,
    /// 必填字段。
    pub required: bool,
    /// 无障碍标签。
    pub label: String,
    /// 表单字段名。
    pub name: String,
    /// 组件尺寸：xs | s | m | l | xl | small | medium | large。
    pub size: String,
}

impl Default for WaRatingState {
    fn default() -> Self {
        Self {
            value: 0.0,
            default_value: 0.0,
            max: 5.0,
            precision: 1.0,
            readonly: false,
            disabled: false,
            required: false,
            label: String::new(),
            name: String::new(),
            size: "m".into(),
        }
    }
}

impl WaRatingState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 将值四舍五入到最近的精度步长。
    fn round_to_precision(&self, value: f64) -> f64 {
        let precision = f64::max(self.precision, 1e-9); // 避免除零
        let multiplier = 1.0 / precision;
        let rounded = (value * multiplier).round() / multiplier;
        rounded.clamp(0.0, self.max)
    }

    /// 获取用于显示的评分值。
    fn display_value(&self) -> f64 {
        self.round_to_precision(self.value)
    }

    /// 根据 size 返回星星的像素大小。
    fn star_size_px(size: &str) -> f64 {
        match size {
            "xs" => 12.0,
            "s" | "small" => 16.0,
            "m" | "medium" => 24.0,
            "l" | "large" => 32.0,
            "xl" => 40.0,
            _ => 24.0, // 默认 m
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaRatingMessage {
    /// 评分值变更。
    Change,
    /// 悬停值变更（wa-hover 事件）。
    Hover,
    /// 验证失败。
    Invalid,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaRating;

impl WidgetSpec for WaRating {
    type State = WaRatingState;
    type Message = WaRatingMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaRating"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaRating")
            .prop("value", PropValue::Float(OrderedFloat(state.value)))
            .prop("max", PropValue::Float(OrderedFloat(state.max)))
            .prop("precision", PropValue::Float(OrderedFloat(state.precision)))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop(
                "label",
                PropValue::str(if state.label.is_empty() {
                    "rating"
                } else {
                    state.label.as_str()
                }),
            )
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("readonly", PropValue::Bool(state.readonly))
            .prop("required", PropValue::Bool(state.required))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaRatingMessage::Change => {
                // Change 事件——值已通过别的路径更新（如 paint_factory 设置），此处不做额外处理
            },
            WaRatingMessage::Hover => {
                // Phase 0 跳过悬停交互
            },
            WaRatingMessage::Invalid => {
                // Phase 0 跳过表单验证
            },
        }
    }

    /// Rating 尺寸由 Taffy 布局决定。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 颜色：未选中灰色、选中黄色（来自 WA CSS 变量）
        let inactive_color = Color::new(0.55, 0.55, 0.55, 1.0); // --symbol-color
        let active_color = Color::new(0.94, 0.73, 0.15, 1.0); // --symbol-color-active (yellow-70)
        let star_filled = "\u{2605}"; // ★ 实心星
        let star_empty = "\u{2606}"; // ☆ 空心星

        let max = state.max.max(1.0); // 至少 1 颗星
        let display_value = state.display_value();
        let star_size = WaRatingState::star_size_px(&state.size);

        let h: f64 = bounds.size.height;
        let w: f64 = bounds.size.width;

        if h < 8.0 || w < 8.0 {
            return; // 太小，无法绘制
        }

        // 星星的实际绘制大小取 bounds 高度和 star_size 的较小值
        let actual_star_size: f64 = star_size.min(h * 0.8);
        let font_size: f32 = actual_star_size as f32;

        // 总星星宽度
        let total_width: f64 = actual_star_size * max;
        // 居中起始位置
        let start_x: f64 = bounds.origin.x + f64::max(0.0, (w - total_width) / 2.0);
        let center_y: f64 = bounds.origin.y + h / 2.0;

        // 逐颗绘制星星
        for i in 0..(max as usize) {
            let idx = i as f64;
            let is_filled = display_value >= idx + 1.0;

            let star_char = if is_filled { star_filled } else { star_empty };
            let color = if is_filled {
                active_color
            } else {
                inactive_color
            };

            // 每颗星的位置
            let star_x = start_x + idx * actual_star_size;
            let star_rect = Rect::new(
                star_x,
                center_y - actual_star_size / 2.0,
                actual_star_size,
                actual_star_size,
            );

            ctx.draw_text(star_char, star_rect, color, font_size);
        }

        // 绘制分值数字（在星星右侧或下方）
        if display_value > 0.0 {
            let value_text = format!("{:.1}", display_value);
            let text_font_size: f32 = (actual_star_size * 0.7) as f32;
            let text_rect = Rect::new(
                start_x + total_width + 4.0,
                center_y - actual_star_size * 0.3,
                actual_star_size * 1.8,
                actual_star_size * 0.6,
            );
            ctx.draw_text(&value_text, text_rect, inactive_color, text_font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label_text = if state.label.is_empty() {
            "rating".to_string()
        } else {
            state.label.clone()
        };

        let value_text = format!("{} of {}", state.display_value() as i32, state.max as i32);

        AccessibilityNode::new(WidgetId::from_u64(0), AccessibilityRole::Slider, Rect::ZERO)
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
        let state = WaRatingState::new();
        assert_eq!(state.value, 0.0);
        assert_eq!(state.max, 5.0);
        assert_eq!(state.precision, 1.0);
        assert!(!state.readonly);
        assert!(!state.disabled);
        assert!(!state.required);
        assert!(state.label.is_empty());
        assert!(state.name.is_empty());
        assert_eq!(state.size, "m");
    }

    #[test]
    fn state_with_custom_max() {
        let state = WaRatingState {
            max: 10.0,
            ..WaRatingState::new()
        };
        assert_eq!(state.max, 10.0);
    }

    #[test]
    fn state_with_value() {
        let state = WaRatingState {
            value: 3.0,
            ..WaRatingState::new()
        };
        assert_eq!(state.value, 3.0);
        assert_eq!(state.display_value(), 3.0);
    }

    #[test]
    fn state_disabled() {
        let state = WaRatingState {
            disabled: true,
            ..WaRatingState::new()
        };
        assert!(state.disabled);
    }

    #[test]
    fn state_readonly() {
        let state = WaRatingState {
            readonly: true,
            ..WaRatingState::new()
        };
        assert!(state.readonly);
    }

    #[test]
    fn state_with_label() {
        let state = WaRatingState {
            label: "Product rating".into(),
            ..WaRatingState::new()
        };
        assert_eq!(state.label, "Product rating");
    }

    #[test]
    fn state_with_name() {
        let state = WaRatingState {
            name: "rating_field".into(),
            ..WaRatingState::new()
        };
        assert_eq!(state.name, "rating_field");
    }

    #[test]
    fn round_to_precision_integer_step() {
        let state = WaRatingState::new(); // precision = 1
        assert_eq!(state.round_to_precision(3.2), 3.0);
        assert_eq!(state.round_to_precision(3.7), 4.0);
        assert_eq!(state.round_to_precision(0.0), 0.0);
        assert_eq!(state.round_to_precision(5.0), 5.0);
        assert_eq!(state.round_to_precision(5.5), 5.0); // clamp to max
    }

    #[test]
    fn round_to_precision_half_step() {
        let state = WaRatingState {
            precision: 0.5,
            max: 5.0,
            ..WaRatingState::new()
        };
        assert_eq!(state.round_to_precision(3.2), 3.0);
        assert_eq!(state.round_to_precision(3.3), 3.5); // 3.3 * 2 = 6.6, round=7, /2=3.5
        assert_eq!(state.round_to_precision(3.7), 3.5); // 3.7 * 2 = 7.4, round=7, /2=3.5
        assert_eq!(state.round_to_precision(3.8), 4.0); // 3.8 * 2 = 7.6, round=8, /2=4.0
        assert_eq!(state.round_to_precision(0.0), 0.0);
    }

    #[test]
    fn round_to_precision_clamp() {
        let state = WaRatingState::new(); // max = 5
        assert_eq!(state.round_to_precision(-1.0), 0.0);
        assert_eq!(state.round_to_precision(10.0), 5.0);
    }

    #[test]
    fn measure_returns_zero() {
        let state = WaRatingState::new();
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let ctx = MeasureContext::default();
        let size = WaRating.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_produces_ops_for_default_state() {
        let state = WaRatingState::new(); // value = 0, max = 5
        let bounds = Rect::new(0.0, 0.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaRating.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 应产生 5 个 DrawText（5 颗空心星）+ 0 个 DrawText（value=0 无数值标签）
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 5, "value=0 应有 5 颗空心星，无数字标签");
    }

    #[test]
    fn paint_produces_ops_for_filled_value() {
        let state = WaRatingState {
            value: 3.0,
            max: 5.0,
            ..WaRatingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaRating.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 5 颗星 + 1 个数字标签 = 6 个 DrawText
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 6, "value=3 应有 5 颗星 + 1 个数字标签");
    }

    #[test]
    fn paint_full_value() {
        let state = WaRatingState {
            value: 5.0,
            max: 5.0,
            ..WaRatingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaRating.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        // 全部 5 颗实心星
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 6, "value=5 应有 5 颗星 + 1 个数字标签");
    }

    #[test]
    fn paint_too_small_bounds_returns_early() {
        let state = WaRatingState {
            value: 3.0,
            ..WaRatingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 3.0, 3.0);
        let mut ctx = PaintContext::new(bounds);
        WaRating.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "过小 bounds 应提前返回");
    }

    #[test]
    fn paint_disabled_shows_value() {
        let state = WaRatingState {
            value: 4.0,
            max: 5.0,
            disabled: true,
            ..WaRatingState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 40.0);
        let mut ctx = PaintContext::new(bounds);
        WaRating.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 6, "disabled 不影响绘制操作数量");
    }

    #[test]
    fn star_size_mapping() {
        assert_eq!(WaRatingState::star_size_px("xs"), 12.0);
        assert_eq!(WaRatingState::star_size_px("s"), 16.0);
        assert_eq!(WaRatingState::star_size_px("small"), 16.0);
        assert_eq!(WaRatingState::star_size_px("m"), 24.0);
        assert_eq!(WaRatingState::star_size_px("medium"), 24.0);
        assert_eq!(WaRatingState::star_size_px("l"), 32.0);
        assert_eq!(WaRatingState::star_size_px("large"), 32.0);
        assert_eq!(WaRatingState::star_size_px("xl"), 40.0);
        assert_eq!(WaRatingState::star_size_px("unknown"), 24.0); // fallback
    }

    #[test]
    fn accessibility_default_state() {
        let state = WaRatingState::new();
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaRating.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("rating"));
        assert_eq!(node.value.as_deref(), Some("0 of 5"));
    }

    #[test]
    fn accessibility_with_custom_label_and_value() {
        let state = WaRatingState {
            label: "Product score".into(),
            value: 4.0,
            max: 5.0,
            ..WaRatingState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaRating.accessibility(&state, &access_ctx);
        assert_eq!(node.label.as_deref(), Some("Product score"));
        assert_eq!(node.value.as_deref(), Some("4 of 5"));
    }

    #[test]
    fn accessibility_with_half_value() {
        let state = WaRatingState {
            value: 3.5,
            max: 5.0,
            precision: 0.5,
            ..WaRatingState::new()
        };
        let access_ctx = AccessContext::new(Rect::ZERO);
        let node = WaRating.accessibility(&state, &access_ctx);
        assert_eq!(node.value.as_deref(), Some("3 of 5")); // display_value 3.5 → as i32 = 3
    }

    #[test]
    fn view_default() {
        let state = WaRatingState::new();
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaRating.view(&state, &ctx);
        assert_eq!(view.widget_type, "rgui_components::WaRating");

        if let Some(PropValue::Float(v)) = view.props.get("value") {
            assert_eq!(v.0, 0.0);
        } else {
            panic!("Expected Float prop 'value'");
        }

        if let Some(PropValue::Float(m)) = view.props.get("max") {
            assert_eq!(m.0, 5.0);
        } else {
            panic!("Expected Float prop 'max'");
        }

        if let Some(PropValue::Str(s)) = view.props.get("label") {
            assert_eq!(s.as_ref(), "rating");
        } else {
            panic!("Expected Str prop 'label'");
        }
    }

    #[test]
    fn view_with_all_props() {
        let state = WaRatingState {
            value: 3.5,
            max: 10.0,
            precision: 0.5,
            readonly: true,
            disabled: true,
            required: true,
            label: "Score".into(),
            name: "rating_name".into(),
            size: "l".into(),
            ..WaRatingState::new()
        };
        let ctx = ViewContext::new(Size::ZERO);
        let view = WaRating.view(&state, &ctx);

        if let Some(PropValue::Float(v)) = view.props.get("value") {
            assert_eq!(v.0, 3.5);
        }
        if let Some(PropValue::Float(m)) = view.props.get("max") {
            assert_eq!(m.0, 10.0);
        }
        if let Some(PropValue::Float(p)) = view.props.get("precision") {
            assert_eq!(p.0, 0.5);
        }
        if let Some(PropValue::Bool(r)) = view.props.get("readonly") {
            assert!(r);
        }
        if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
            assert!(d);
        }
        if let Some(PropValue::Bool(rq)) = view.props.get("required") {
            assert!(rq);
        }
        if let Some(PropValue::Str(s)) = view.props.get("label") {
            assert_eq!(s.as_ref(), "Score");
        }
        if let Some(PropValue::Str(n)) = view.props.get("name") {
            assert_eq!(n.as_ref(), "rating_name");
        }
        if let Some(PropValue::Str(sz)) = view.props.get("size") {
            assert_eq!(sz.as_ref(), "l");
        }
    }

    #[test]
    fn update_change_message() {
        let mut state = WaRatingState {
            value: 3.0,
            ..WaRatingState::new()
        };
        let mut ctx = UpdateContext::default();
        WaRating.update(WaRatingMessage::Change, &mut state, &mut ctx);
        assert_eq!(state.value, 3.0); // Change 不修改值，值由 paint_factory 设置
    }

    #[test]
    fn update_hover_message() {
        let mut state = WaRatingState::new();
        let mut ctx = UpdateContext::default();
        WaRating.update(WaRatingMessage::Hover, &mut state, &mut ctx);
        // Phase 0 跳过悬停，无副作用
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaRating.name(), "rgui_components::WaRating");
    }

    #[test]
    fn display_value_rounds() {
        let state = WaRatingState {
            value: 3.7,
            precision: 1.0,
            max: 5.0,
            ..WaRatingState::new()
        };
        assert_eq!(state.display_value(), 4.0); // rounds to 4
    }

    #[test]
    fn default_value_field() {
        let state = WaRatingState {
            default_value: 3.0,
            value: 0.0,
            ..WaRatingState::new()
        };
        assert_eq!(state.default_value, 3.0);
        // default_value 用于表单重置，不影响 display_value
        assert_eq!(state.display_value(), 0.0);
    }
}
