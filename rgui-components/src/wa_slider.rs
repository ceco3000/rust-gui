/// Translated from Web Awesome wa-slider (aka Range)
/// Original license: MIT
/// Copyright (c) Font Awesome
use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
// 以下 traits 由派生宏实现，test 中需要 in scope
use ordered_float::OrderedFloat;
#[allow(unused_imports)]
use rgui_core::traits::{AppMessage, PersistState};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage as AppMsg, PersistState as Persist};

// ============================================================================
// State
// ============================================================================

/// Web Awesome wa-slider 组件状态。
///
/// Slider (Range) 是滑块组件，用户拖动拇指沿轨道选择数值。
/// 支持单值和双拇指范围模式、水平/垂直方向、标记点、步长。
///
/// 简化项：
/// - range 双拇指模式简化为 TODO（rgui 无 DraggableElement 拖拽系统）
/// - tooltip 弹层跳过（WT55-WT61 弹层组件 P2）
/// - valueFormatter 跳过（JS 函数不可移植）
/// - 跳过 withLabel/withHint SSR 属性
/// - 跳过 autofocus（框架暂未支持）
/// - FormField trait impl 暂时跳过
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaSliderState {
    /// 当前值
    pub value: f64,
    /// 默认值（表单重置时使用）
    pub default_value: f64,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 步长
    pub step: f64,
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 方向：horizontal | vertical
    pub orientation: String,
    /// 禁用状态
    pub disabled: bool,
    /// 只读状态
    pub readonly: bool,
    /// 标签文本
    pub label: String,
    /// 提示文本
    pub hint: String,
    /// 填充起始偏移值
    pub indicator_offset: Option<f64>,
    /// 显示刻度标记
    pub with_markers: bool,
}

impl Default for WaSliderState {
    fn default() -> Self {
        Self {
            value: 0.0,
            default_value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            size: "m".into(),
            orientation: "horizontal".into(),
            disabled: false,
            readonly: false,
            label: String::new(),
            hint: String::new(),
            indicator_offset: None,
            with_markers: false,
        }
    }
}

impl WaSliderState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    /// Clamp value to [min, max] and round to step
    fn clamp_to_step(&self, value: f64) -> f64 {
        let step = self.step.max(f64::EPSILON);
        let min = self.min;
        let max = self.max;
        let mut v = (value / step).round() * step;
        if v < min {
            v = min;
        }
        if v > max {
            v = max;
        }
        v
    }

    /// 计算值在 [min, max] 范围内的百分比位置 (0-100)
    fn percentage(&self, value: f64) -> f64 {
        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            return 0.0;
        }
        ((value - self.min) / range * 100.0).clamp(0.0, 100.0)
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaSliderMessage {
    /// 失去焦点
    Blur,
    /// 值已提交
    Change,
    /// 获得焦点
    Focus,
    /// 接收输入
    Input,
    /// 验证失败
    Invalid,
    /// 上一步（键盘 ArrowUp / ArrowRight）
    StepUp,
    /// 下一步（键盘 ArrowDown / ArrowLeft）
    StepDown,
    /// 跳到最小值（Home）
    GoMin,
    /// 跳到最大值（End）
    GoMax,
    /// 跳 10%（PageUp）
    PageUp,
    /// 跳 10%（PageDown）
    PageDown,
}

// ============================================================================
// Helper functions
// ============================================================================

/// 根据 size 属性的像素高度映射
fn slider_track_thickness(size: &str) -> f64 {
    match size {
        "xs" => 4.0,
        "s" | "small" => 6.0,
        "l" | "large" => 12.0,
        "xl" => 16.0,
        _ => 8.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的 thumb 尺寸映射
fn slider_thumb_size(size: &str) -> f64 {
    match size {
        "xs" => 12.0,
        "s" | "small" => 16.0,
        "l" | "large" => 28.0,
        "xl" => 36.0,
        _ => 22.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的字体大小映射
fn slider_font_size(size: &str) -> f64 {
    match size {
        "xs" => 10.0,
        "s" | "small" => 12.0,
        "l" | "large" => 16.0,
        "xl" => 20.0,
        _ => 14.0, // "m" | "medium" (默认)
    }
}

/// 占位色（轨道背景）
const TRACK_COLOR: Color = Color::new(0.82, 0.82, 0.82, 1.0);
/// 指示器填充色
const INDICATOR_COLOR: Color = Color::new(0.2, 0.5, 0.9, 1.0);
/// 拇指填充色
const THUMB_COLOR: Color = Color::new(0.2, 0.5, 0.9, 1.0);
/// 拇指边框色
const THUMB_BORDER_COLOR: Color = Color::WHITE;
/// 禁用态颜色
const DISABLED_COLOR: Color = Color::new(0.7, 0.7, 0.7, 0.6);
/// 标签颜色
const LABEL_COLOR: Color = Color::new(0.2, 0.2, 0.2, 1.0);

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaSlider;

impl WidgetSpec for WaSlider {
    type State = WaSliderState;
    type Message = WaSliderMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaSlider"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaSlider")
            .prop("value", PropValue::Float(OrderedFloat(state.value)))
            .prop("min", PropValue::Float(OrderedFloat(state.min)))
            .prop("max", PropValue::Float(OrderedFloat(state.max)))
            .prop("step", PropValue::Float(OrderedFloat(state.step)))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("orientation", PropValue::str(state.orientation.as_str()))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("readonly", PropValue::Bool(state.readonly))
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("hint", PropValue::str(state.hint.as_str()))
            .prop("with-markers", PropValue::Bool(state.with_markers));
        if let Some(offset) = state.indicator_offset {
            v = v.prop("indicator-offset", PropValue::Float(OrderedFloat(offset)));
        }
        v
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        if state.disabled || state.readonly {
            return;
        }
        match msg {
            WaSliderMessage::StepUp => {
                let new_val = state.clamp_to_step(state.value + state.step);
                if (new_val - state.value).abs() > f64::EPSILON {
                    state.value = new_val;
                }
            },
            WaSliderMessage::StepDown => {
                let new_val = state.clamp_to_step(state.value - state.step);
                if (new_val - state.value).abs() > f64::EPSILON {
                    state.value = new_val;
                }
            },
            WaSliderMessage::GoMin => {
                state.value = state.min;
            },
            WaSliderMessage::GoMax => {
                state.value = state.max;
            },
            WaSliderMessage::PageUp => {
                let step_up = state.value + (state.max - state.min) / 10.0;
                let new_val = state.clamp_to_step(step_up.max(state.value + state.step));
                if (new_val - state.value).abs() > f64::EPSILON {
                    state.value = new_val;
                }
            },
            WaSliderMessage::PageDown => {
                let step_down = state.value - (state.max - state.min) / 10.0;
                let new_val = state.clamp_to_step(step_down.min(state.value - state.step));
                if (new_val - state.value).abs() > f64::EPSILON {
                    state.value = new_val;
                }
            },
            WaSliderMessage::Change | WaSliderMessage::Input => {
                // 值由框架通过 props/rhai 更新，这里只标记已处理
            },
            WaSliderMessage::Blur | WaSliderMessage::Focus | WaSliderMessage::Invalid => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let track_thickness = slider_track_thickness(&state.size);
        let thumb_size = slider_thumb_size(&state.size);
        let font_size = slider_font_size(&state.size);

        // label 高度（如果有标签）
        let label_height = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.5
        };
        // hint 高度（如果有提示）
        let hint_height = if state.hint.is_empty() {
            0.0
        } else {
            font_size * 1.2
        };

        let (w, h) = if state.orientation == "vertical" {
            // 垂直：宽度由 thumb + track 决定，高度较长
            let content_w = thumb_size.max(track_thickness) + 8.0;
            let content_h = 200.0 + label_height + hint_height;
            (content_w, content_h)
        } else {
            // 水平：宽度较长，高度由 thumb + track + label/hint 决定
            let content_w = 200.0;
            let content_h = thumb_size.max(track_thickness) + 8.0 + label_height + hint_height;
            (content_w, content_h)
        };

        Size::new(
            w.clamp(c.min_width, c.max_width),
            h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let track_thickness = slider_track_thickness(&state.size);
        let thumb_size = slider_thumb_size(&state.size);
        let font_size = slider_font_size(&state.size);
        let is_disabled = state.disabled;

        // 颜色：根据禁用状态调整
        let track_color = if is_disabled {
            DISABLED_COLOR
        } else {
            TRACK_COLOR
        };
        let indicator_color = if is_disabled {
            DISABLED_COLOR
        } else {
            INDICATOR_COLOR
        };
        let thumb_color = if is_disabled {
            DISABLED_COLOR
        } else {
            THUMB_COLOR
        };
        let thumb_border = if is_disabled {
            Color::new(0.8, 0.8, 0.8, 0.6)
        } else {
            THUMB_BORDER_COLOR
        };
        let label_color = if is_disabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            LABEL_COLOR
        };

        // ── 标签 ──
        let label_offset_y = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.5
        };
        if !state.label.is_empty() {
            let label_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                label_offset_y,
            );
            ctx.draw_text(
                &state.label,
                label_rect,
                label_color,
                font_size as f32 * 0.85,
            );
        }

        // ── hint ──
        let hint_height = if state.hint.is_empty() {
            0.0
        } else {
            font_size * 1.2
        };
        let slider_area_y = bounds.origin.y + label_offset_y;
        let slider_area_h = thumb_size.max(track_thickness) + 8.0;

        if state.orientation == "vertical" {
            // ── 垂直布局：轨道从上到下，thumb 在垂直位置 ──
            let track_x = bounds.origin.x + (bounds.size.width - track_thickness) / 2.0;
            let track_y = slider_area_y + 4.0;
            let track_h = bounds.size.height - label_offset_y - hint_height - 8.0;
            let track_rect = Rect::new(track_x, track_y, track_thickness, track_h);

            // 轨道背景
            ctx.fill_rect(track_rect, track_color, (track_thickness / 2.0) as f32);

            // 填充指示器（从底部到 thumb 位置）
            let pct = state.percentage(state.value) / 100.0;
            let indicator_offset = state.indicator_offset.map_or(state.min, |o| o);
            let start_pct = state.percentage(indicator_offset) / 100.0;

            let fill_bottom = track_y + track_h - start_pct.min(pct) * track_h;
            let fill_top = track_y + track_h - start_pct.max(pct) * track_h;
            let fill_h = (fill_bottom - fill_top).max(1.0);
            let fill_rect = Rect::new(track_x, fill_top, track_thickness, fill_h);
            ctx.fill_rect(fill_rect, indicator_color, (track_thickness / 2.0) as f32);

            // 拇指（圆形）
            let thumb_cy = track_y + track_h - pct * track_h;
            let thumb_cx = track_x + track_thickness / 2.0;
            let thumb_radius = thumb_size / 2.0;
            let thumb_rect = Rect::new(
                thumb_cx - thumb_radius,
                thumb_cy - thumb_radius,
                thumb_size,
                thumb_size,
            );
            // 拇指边框（稍大一点的圆）
            let border_rect = Rect::new(
                thumb_cx - thumb_radius - 1.0,
                thumb_cy - thumb_radius - 1.0,
                thumb_size + 2.0,
                thumb_size + 2.0,
            );
            ctx.fill_rect(border_rect, thumb_border, (thumb_radius + 1.0) as f32);
            ctx.fill_rect(thumb_rect, thumb_color, thumb_radius as f32);

            // 刻度标记（可选）
            if state.with_markers {
                let n_steps = ((state.max - state.min) / state.step).round() as i32;
                for i in 1..n_steps {
                    let marker_pct = (i as f64 * state.step) / (state.max - state.min);
                    let marker_y = track_y + track_h - marker_pct * track_h;
                    let marker_size: f64 = 3.0;
                    let marker_rect = Rect::new(
                        track_x - marker_size - 2.0,
                        marker_y - marker_size / 2.0,
                        marker_size,
                        marker_size,
                    );
                    ctx.fill_rect(marker_rect, track_color, (marker_size / 2.0) as f32);
                }
            }
        } else {
            // ── 水平布局：轨道从左到右，thumb 在水平位置 ──
            let track_y = slider_area_y + (slider_area_h - track_thickness) / 2.0;
            let track_w = bounds.size.width - 8.0;
            let track_x = bounds.origin.x + 4.0;
            let track_rect = Rect::new(track_x, track_y, track_w, track_thickness);

            // 轨道背景
            ctx.fill_rect(track_rect, track_color, (track_thickness / 2.0) as f32);

            // 填充指示器
            let pct = state.percentage(state.value) / 100.0;
            let indicator_offset = state.indicator_offset.map_or(state.min, |o| o);
            let start_pct = state.percentage(indicator_offset) / 100.0;

            let fill_left = track_x + start_pct.min(pct) * track_w;
            let fill_right = track_x + start_pct.max(pct) * track_w;
            let fill_w = (fill_right - fill_left).max(1.0);
            let fill_rect = Rect::new(fill_left, track_y, fill_w, track_thickness);
            ctx.fill_rect(fill_rect, indicator_color, (track_thickness / 2.0) as f32);

            // 拇指（圆形）
            let thumb_cx = track_x + pct * track_w;
            let thumb_cy = track_y + track_thickness / 2.0;
            let thumb_radius = thumb_size / 2.0;
            let thumb_rect = Rect::new(
                thumb_cx - thumb_radius,
                thumb_cy - thumb_radius,
                thumb_size,
                thumb_size,
            );
            // 拇指边框
            let border_rect = Rect::new(
                thumb_cx - thumb_radius - 1.0,
                thumb_cy - thumb_radius - 1.0,
                thumb_size + 2.0,
                thumb_size + 2.0,
            );
            ctx.fill_rect(border_rect, thumb_border, (thumb_radius + 1.0) as f32);
            ctx.fill_rect(thumb_rect, thumb_color, thumb_radius as f32);

            // 刻度标记（可选）
            if state.with_markers {
                let n_steps = ((state.max - state.min) / state.step).round() as i32;
                for i in 1..n_steps {
                    let marker_pct = (i as f64 * state.step) / (state.max - state.min);
                    let marker_x = track_x + marker_pct * track_w;
                    let marker_size: f64 = 3.0;
                    let marker_rect = Rect::new(
                        marker_x - marker_size / 2.0,
                        track_y - marker_size - 2.0,
                        marker_size,
                        marker_size,
                    );
                    ctx.fill_rect(marker_rect, track_color, (marker_size / 2.0) as f32);
                }
            }
        }

        // ── hint 文本 ──
        if !state.hint.is_empty() {
            let hint_y = slider_area_y + slider_area_h + 4.0;
            let hint_rect = Rect::new(bounds.origin.x, hint_y, bounds.size.width, hint_height);
            let hint_color = Color::new(0.5, 0.5, 0.5, 1.0);
            ctx.draw_text(
                &state.hint,
                hint_rect,
                hint_color,
                (font_size * 0.75) as f32,
            );
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label = if state.label.is_empty() {
            "slider"
        } else {
            state.label.as_str()
        };
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
    fn name() {
        assert_eq!(WaSlider.name(), "rgui_components::WaSlider");
    }

    #[test]
    fn default_state_values() {
        let state = WaSliderState::default();
        assert!((state.value - 0.0).abs() < f64::EPSILON);
        assert!((state.min - 0.0).abs() < f64::EPSILON);
        assert!((state.max - 100.0).abs() < f64::EPSILON);
        assert!((state.step - 1.0).abs() < f64::EPSILON);
        assert_eq!(state.orientation, "horizontal");
        assert_eq!(state.size, "m");
        assert!(!state.disabled);
        assert!(!state.readonly);
        assert!(!state.with_markers);
        assert!(state.indicator_offset.is_none());
    }

    #[test]
    fn new_sets_label() {
        let state = WaSliderState::new("Volume");
        assert_eq!(state.label, "Volume");
    }

    #[test]
    fn clamp_to_step_basic() {
        let state = WaSliderState {
            min: 0.0,
            max: 100.0,
            step: 5.0,
            ..WaSliderState::default()
        };
        assert!((state.clamp_to_step(7.0) - 5.0).abs() < f64::EPSILON);
        assert!((state.clamp_to_step(8.0) - 10.0).abs() < f64::EPSILON);
        assert!((state.clamp_to_step(-10.0) - 0.0).abs() < f64::EPSILON);
        assert!((state.clamp_to_step(150.0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn clamp_to_step_exact() {
        let state = WaSliderState {
            min: 0.0,
            max: 100.0,
            step: 10.0,
            ..WaSliderState::default()
        };
        assert!((state.clamp_to_step(30.0) - 30.0).abs() < f64::EPSILON);
        assert!((state.clamp_to_step(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((state.clamp_to_step(100.0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentage_basic() {
        let state = WaSliderState {
            min: 0.0,
            max: 100.0,
            ..WaSliderState::default()
        };
        assert!((state.percentage(50.0) - 50.0).abs() < f64::EPSILON);
        assert!((state.percentage(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((state.percentage(100.0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentage_offset_range() {
        let state = WaSliderState {
            min: 20.0,
            max: 80.0,
            ..WaSliderState::default()
        };
        assert!((state.percentage(50.0) - 50.0).abs() < f64::EPSILON);
        assert!((state.percentage(20.0) - 0.0).abs() < f64::EPSILON);
        assert!((state.percentage(80.0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn view_has_default_props() {
        let state = WaSliderState::new("Volume");
        let v = WaSlider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
        assert!(v.props.contains_key("value"));
        assert!(v.props.contains_key("min"));
        assert!(v.props.contains_key("max"));
        assert!(v.props.contains_key("step"));
        assert!(v.props.contains_key("orientation"));
        assert!(v.props.contains_key("size"));
    }

    #[test]
    fn view_has_disabled_prop() {
        let mut state = WaSliderState::new("Volume");
        state.disabled = true;
        let v = WaSlider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_has_readonly_prop() {
        let mut state = WaSliderState::new("Volume");
        state.readonly = true;
        let v = WaSlider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("readonly"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_orientation_vertical() {
        let mut state = WaSliderState::new("Volume");
        state.orientation = "vertical".into();
        let v = WaSlider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("orientation"),
            Some(&PropValue::Str(std::sync::Arc::from("vertical")))
        );
    }

    #[test]
    fn view_with_markers() {
        let mut state = WaSliderState::new("Volume");
        state.with_markers = true;
        let v = WaSlider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("with-markers"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_indicator_offset() {
        let mut state = WaSliderState::new("Volume");
        state.indicator_offset = Some(25.0);
        let v = WaSlider.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("indicator-offset"));
    }

    #[test]
    fn update_step_up_increases_value() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        state.step = 5.0;
        WaSlider.update(
            WaSliderMessage::StepUp,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 55.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_step_down_decreases_value() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        state.step = 5.0;
        WaSlider.update(
            WaSliderMessage::StepDown,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 45.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_step_down_stops_at_min() {
        let mut state = WaSliderState::new("Volume");
        state.value = 5.0;
        state.step = 10.0;
        state.min = 0.0;
        WaSlider.update(
            WaSliderMessage::StepDown,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_step_up_stops_at_max() {
        let mut state = WaSliderState::new("Volume");
        state.value = 95.0;
        state.step = 10.0;
        state.max = 100.0;
        WaSlider.update(
            WaSliderMessage::StepUp,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_go_min() {
        let mut state = WaSliderState::new("Volume");
        state.value = 75.0;
        state.min = 10.0;
        WaSlider.update(
            WaSliderMessage::GoMin,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_go_max() {
        let mut state = WaSliderState::new("Volume");
        state.value = 25.0;
        state.max = 90.0;
        WaSlider.update(
            WaSliderMessage::GoMax,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_disabled_does_nothing() {
        let mut state = WaSliderState::new("Volume");
        state.disabled = true;
        state.value = 50.0;
        WaSlider.update(
            WaSliderMessage::StepUp,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_readonly_does_nothing() {
        let mut state = WaSliderState::new("Volume");
        state.readonly = true;
        state.value = 50.0;
        WaSlider.update(
            WaSliderMessage::StepUp,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!((state.value - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaSliderState::new("Volume");
        WaSlider.update(
            WaSliderMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_focus_is_handled() {
        let mut state = WaSliderState::new("Volume");
        WaSlider.update(
            WaSliderMessage::Focus,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_invalid_is_handled() {
        let mut state = WaSliderState::new("Volume");
        WaSlider.update(
            WaSliderMessage::Invalid,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_horizontal() {
        let state = WaSliderState::new("Volume");
        let size = WaSlider.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(
            size.width >= 100.0,
            "水平 slider 应 ≥ 100px 宽，实际 {size:?}"
        );
        assert!(
            size.height >= 20.0,
            "水平 slider 应 ≥ 20px 高，实际 {size:?}"
        );
    }

    #[test]
    fn measure_vertical() {
        let mut state = WaSliderState::new("Volume");
        state.orientation = "vertical".into();
        let size = WaSlider.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(
            size.width >= 10.0,
            "垂直 slider 应 ≥ 10px 宽，实际 {size:?}"
        );
        assert!(
            size.height >= 100.0,
            "垂直 slider 应 ≥ 100px 高，实际 {size:?}"
        );
    }

    #[test]
    fn measure_size_xs_smaller() {
        let mut state = WaSliderState::new("Small");
        state.size = "xs".into();
        let xs = WaSlider.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        state.size = "xl".into();
        let xl = WaSlider.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(xs.height < xl.height, "xs 应比 xl 矮，xs={xs:?} xl={xl:?}");
    }

    #[test]
    fn paint_horizontal_produces_ops() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaSlider.paint(&state, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 3,
            "应至少绘制轨道+填充+拇指，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_vertical_produces_ops() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        state.orientation = "vertical".into();
        let bounds = Rect::new(0.0, 0.0, 80.0, 300.0);
        let mut ctx = PaintContext::new(bounds);
        WaSlider.paint(&state, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 3,
            "垂直模式应至少绘制轨道+填充+拇指，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_disabled_shows_muted() {
        let mut state = WaSliderState::new("Volume");
        state.disabled = true;
        state.value = 50.0;
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaSlider.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_with_label_produces_text() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        let bounds = Rect::new(0.0, 0.0, 320.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaSlider.paint(&state, bounds, &mut ctx);
        // label + track + indicator + thumb >= 4
        assert!(
            ctx.op_count() >= 4,
            "带标签应至少 4 个操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_with_hint_produces_text() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        state.hint = "Adjust volume level".into();
        let bounds = Rect::new(0.0, 0.0, 320.0, 80.0);
        let mut ctx = PaintContext::new(bounds);
        WaSlider.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 4);
    }

    #[test]
    fn paint_with_markers_produces_more_ops() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        state.min = 0.0;
        state.max = 100.0;
        state.step = 25.0;
        state.with_markers = true;
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaSlider.paint(&state, bounds, &mut ctx);
        // track + indicator + thumb + 3 markers (at 25/50/75) >= 6
        assert!(
            ctx.op_count() >= 6,
            "带标记应至少 6 个操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_with_indicator_offset() {
        let mut state = WaSliderState::new("Volume");
        state.value = 50.0;
        state.indicator_offset = Some(25.0);
        let bounds = Rect::new(0.0, 0.0, 320.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaSlider.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaSliderMessage::Change.message_name(), "change");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaSliderState::schema_name(), "WaSliderState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaSliderState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaSliderState>());
    }

    #[test]
    fn accessibility_label() {
        let state = WaSliderState::new("Volume");
        let node = WaSlider.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Volume"));
    }

    #[test]
    fn accessibility_no_label_fallback() {
        let state = WaSliderState::default();
        let node = WaSlider.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("slider"));
    }

    #[test]
    fn percentage_zero_range() {
        let state = WaSliderState {
            min: 50.0,
            max: 50.0,
            ..WaSliderState::default()
        };
        assert!((state.percentage(50.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn clamp_to_step_tiny_step() {
        let state = WaSliderState {
            min: 0.0,
            max: 100.0,
            step: 0.1,
            ..WaSliderState::default()
        };
        let v = state.clamp_to_step(50.05);
        // f64 rounding: 50.05/0.1 ≈ 500.5 → round → 501 → *0.1 ≈ 50.1
        // But floating point may give 50.0 or 50.1, both close enough
        let diff = (v - 50.1).abs().min((v - 50.0).abs());
        assert!(diff < 0.05, "expected near 50.0 or 50.1, got {v}");
    }
}
