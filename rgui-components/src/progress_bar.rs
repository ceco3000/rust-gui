//! ProgressBar 组件——进度条。
//!
//! 显示任务完成进度（0.0 ~ 1.0），为只读组件，无交互行为。
//! 发送 `ProgressBarMessage::NoOp`（占位消息）。

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::{AppMessage as Am, PersistState as Ps, WidgetSpec};
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_macros::{AppMessage, PersistState};

/// ProgressBar 业务状态。
///
/// 包含进度值（0.0 ~ 1.0）和标签文本。
#[derive(Debug, Clone, Default, serde::Serialize, PersistState)]
pub struct ProgressBarState {
    /// 进度值，范围 0.0 ~ 1.0。
    pub value: f64,
    /// 标签文本。
    pub label: String,
}
impl ProgressBarState {
    /// 创建新的 ProgressBarState，指定进度值（自动夹到 0.0 ~ 1.0）。
    #[must_use]
    pub fn new(v: f64) -> Self {
        Self {
            value: v.clamp(0.0, 1.0),
            ..Self::default()
        }
    }
}

/// ProgressBar 消息类型（占位）。
///
/// ProgressBar 为只读组件，提供此枚举以满足 `WidgetSpec` 的关联类型约束。
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum ProgressBarMessage {
    NoOp,
}

/// ProgressBar 组件（unit struct）。
///
/// 实现 [`WidgetSpec`] trait。用于进度展示场景。
pub struct ProgressBar;
impl WidgetSpec for ProgressBar {
    type State = ProgressBarState;
    type Message = ProgressBarMessage;
    fn name(&self) -> &'static str {
        "rgui_components::ProgressBar"
    }
    fn view(&self, s: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("ProgressBar").prop(
            "percent",
            PropValue::Float(ordered_float::OrderedFloat(
                (s.value * 100.0).clamp(0.0, 100.0),
            )),
        )
    }
    fn update(&self, _: Self::Message, _: &mut Self::State, _: &mut UpdateContext) {}
    fn measure(&self, _: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::new(
            200_f64.clamp(c.min_width, c.max_width),
            20_f64.clamp(c.min_height, c.max_height),
        )
    }
    fn paint(&self, s: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 轨道背景
        ctx.fill_rect(bounds, Color::new(0.25, 0.25, 0.30, 1.0), 4.0);

        // 填充进度
        let fill_w = (bounds.size.width * s.value.clamp(0.0, 1.0)).max(0.0);
        if fill_w > 0.0 {
            ctx.fill_rect(
                Rect::new(bounds.origin.x, bounds.origin.y, fill_w, bounds.size.height),
                Color::new(0.20, 0.55, 0.95, 1.0),
                4.0,
            );
        }

        // 百分比文本（对齐 Button：水平内边距 + 字号按高度比例）
        if !s.label.is_empty() {
            let pad = 4.0;
            let text_bounds = Rect::new(
                bounds.origin.x + pad,
                bounds.origin.y,
                bounds.size.width - pad * 2.0,
                bounds.size.height,
            );
            let font_size = bounds.size.height as f32 * 0.8;
            ctx.draw_text(&s.label, text_bounds, Color::new(0.9, 0.9, 0.95, 1.0), font_size);
        }
    }
    fn accessibility(&self, s: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(format!("{:.0}%", s.value * 100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamped() {
        let s = ProgressBarState::new(1.5);
        assert!((s.value - 1.0).abs() < f64::EPSILON);
    }
    #[test]
    fn view_pct() {
        let v = ProgressBar.view(
            &ProgressBarState::new(0.75),
            &ViewContext::new(Size::new(800.0, 600.0)),
        );
        assert!(v.props.contains_key("percent"));
    }

    #[test]
    fn paint_progress() {
        let mut s = ProgressBarState::new(0.5);
        s.label = "50%".into();
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        ProgressBar.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 2, "应绘制轨道 + 填充");
    }

    #[test]
    fn paint_empty_progress() {
        let s = ProgressBarState::new(0.0);
        let bounds = Rect::new(0.0, 0.0, 200.0, 20.0);
        let mut ctx = PaintContext::new(bounds);
        ProgressBar.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "至少绘制轨道");
    }
}
