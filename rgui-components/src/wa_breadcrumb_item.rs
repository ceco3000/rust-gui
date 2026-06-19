/// Translated from Web Awesome wa-breadcrumb-item
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

/// Web Awesome wa-breadcrumb-item 组件状态。
///
/// 面包屑导航中的单个项，表示层次结构中的一个级别。
/// 标签文本通过 children 传入；href 非空时渲染为链接。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaBreadcrumbItemState {
    /// 面包屑项的显示文本
    pub label: String,
    /// 可选链接 URL
    pub href: String,
    /// 链接打开目标（_blank/_parent/_self/_top）
    pub target: String,
    /// 链接 rel 属性
    pub rel: String,
    /// 是否显示分隔符（最后一个面包屑项应设为 false）
    pub separator: bool,
}

impl WaBreadcrumbItemState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            separator: true,
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaBreadcrumbItemMessage {
    /// 占位消息——BreadcrumbItem 无交互事件
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaBreadcrumbItem;

impl WidgetSpec for WaBreadcrumbItem {
    type State = WaBreadcrumbItemState;
    type Message = WaBreadcrumbItemMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaBreadcrumbItem"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaBreadcrumbItem")
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("href", PropValue::str(state.href.as_str()))
            .prop("target", PropValue::str(state.target.as_str()))
            .prop("rel", PropValue::str(state.rel.as_str()))
            .prop("separator", PropValue::Bool(state.separator))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaBreadcrumbItemMessage::NoOp => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let char_count = state.label.chars().count().max(1) as f64;
        let separator_width: f64 = if state.separator { 24.0 } else { 0.0 };
        let tw = (char_count * 9.0 + 4.0 + separator_width).max(40.0);
        let th: f64 = 24.0;
        Size::new(
            tw.clamp(c.min_width, c.max_width),
            th.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let font_size: f32 = 14.0;
        // WA 默认颜色 --wa-color-text-link（链接蓝色）
        let link_color = Color::new(0.20, 0.50, 0.90, 1.0);
        // 分隔符使用安静色 --wa-color-text-quiet
        let separator_color = Color::new(0.50, 0.50, 0.55, 1.0);

        let separator_width: f64 = if state.separator { 20.0 } else { 0.0 };
        let text_width = bounds.size.width - separator_width;

        // 绘制标签文本
        let text_bounds = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            text_width,
            bounds.size.height,
        );
        ctx.draw_text(&state.label, text_bounds, link_color, font_size);

        // 绘制分隔符 ">"
        if state.separator {
            let sep_bounds = Rect::new(
                bounds.origin.x + text_width + 4.0,
                bounds.origin.y,
                16.0,
                bounds.size.height,
            );
            ctx.draw_text(">", sep_bounds, separator_color, font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(state.label.as_str())
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
        assert_eq!(WaBreadcrumbItem.name(), "rgui_components::WaBreadcrumbItem");
    }

    #[test]
    fn view_has_label() {
        let state = WaBreadcrumbItemState::new("Home");
        let v = WaBreadcrumbItem.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_default_separator_true() {
        let state = WaBreadcrumbItemState::new("Home");
        let v = WaBreadcrumbItem.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("separator"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_no_separator() {
        let mut state = WaBreadcrumbItemState::new("Current");
        state.separator = false;
        let v = WaBreadcrumbItem.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("separator"), Some(&PropValue::Bool(false)));
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaBreadcrumbItemState::new("Home");
        WaBreadcrumbItem.update(
            WaBreadcrumbItemMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaBreadcrumbItemState::new("A");
        let size = WaBreadcrumbItem.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 40.0, "宽度应 ≥ 40px，实际 {size:?}");
        assert!(size.height >= 24.0, "高度应 ≥ 24px，实际 {size:?}");
    }

    #[test]
    fn measure_with_separator_wider() {
        let with_sep = WaBreadcrumbItemState::new("Home"); // separator=true
        let mut no_sep = WaBreadcrumbItemState::new("Home");
        no_sep.separator = false;
        let w = WaBreadcrumbItem.measure(
            &with_sep,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        let n = WaBreadcrumbItem.measure(
            &no_sep,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(w.width > n.width, "带分隔符应更宽");
    }

    #[test]
    fn paint_produces_text_ops() {
        let state = WaBreadcrumbItemState::new("Home");
        let bounds = Rect::new(0.0, 0.0, 200.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaBreadcrumbItem.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert!(text_count >= 2, "应绘制标签文本 + 分隔符，实际 {text_count}");
    }

    #[test]
    fn paint_no_separator_single_text() {
        let mut state = WaBreadcrumbItemState::new("Current");
        state.separator = false;
        let bounds = Rect::new(0.0, 0.0, 200.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaBreadcrumbItem.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        let text_count = ops
            .iter()
            .filter(|op| matches!(op, rgui_core::context::PaintOp::DrawText { .. }))
            .count();
        assert_eq!(text_count, 1, "无分隔符时仅绘制标签文本");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(
            WaBreadcrumbItemMessage::NoOp.message_name(),
            "no_op"
        );
    }

    #[test]
    fn derive_state() {
        assert_eq!(
            WaBreadcrumbItemState::schema_name(),
            "WaBreadcrumbItemState"
        );
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaBreadcrumbItemState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaBreadcrumbItemState>());
    }
}
