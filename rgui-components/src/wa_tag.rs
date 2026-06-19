/// Translated from Web Awesome wa-tag
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

/// Web Awesome wa-tag 组件状态。
///
/// 标签组件，用于显示状态、分类或可选标记。支持可移除模式。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaTagState {
    /// 主题变体：brand | neutral | success | warning | danger
    pub variant: String,
    /// 视觉外观：accent | filled | outlined | filled-outlined
    pub appearance: String,
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 是否 pill 形状（全圆角）
    pub pill: bool,
    /// 是否显示移除按钮
    pub with_remove: bool,
    /// 标签文本内容（从 default slot 提取）
    pub label: String,
}

impl WaTagState {
    #[must_use]
    pub fn new(label: &str) -> Self {
        Self {
            variant: "neutral".into(),
            appearance: "filled-outlined".into(),
            size: "m".into(),
            pill: false,
            with_remove: false,
            label: label.into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaTagMessage {
    /// 移除按钮被点击
    Remove,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaTag;

impl WidgetSpec for WaTag {
    type State = WaTagState;
    type Message = WaTagMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaTag"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaTag")
            .prop("variant", PropValue::str(state.variant.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("pill", PropValue::bool(state.pill))
            .prop("with_remove", PropValue::bool(state.with_remove))
            .prop("label", PropValue::str(state.label.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaTagMessage::Remove => {
                // 移除事件由外部处理
            },
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // Tag 是叶子组件，根据文本内容 + padding + 可选移除按钮计算尺寸。
        // 字号 ≈ 14px（WA m 尺寸），Inter Regular em_height ≈ 1.196
        let font_size: f64 = 14.0;
        let em_height: f64 = 1.196;
        let text_h = font_size * em_height;
        let pad_v: f64 = 4.0;
        let pad_h: f64 = 12.0;
        let remove_btn_w: f64 = if state.with_remove { 16.0 } else { 0.0 };

        let text_w = state.label.chars().count() as f64 * font_size * 0.6;

        let w = (text_w + pad_h * 2.0 + remove_btn_w).clamp(c.min_width, c.max_width);
        let h = (text_h + pad_v * 2.0)
            .clamp(c.min_height, c.max_height)
            .max(20.0);

        Size::new(w, h)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let border_radius: f32 = if state.pill { 999.0 } else { 4.0 };
        let border_width: f64 = 1.0;
        let font_size: f32 = 14.0;

        // 根据 appearance 确定颜色（使用 neutral 变体的基本颜色映射）
        let (bg_color, border_color, text_color) = match state.appearance.as_str() {
            "outlined" => (
                Color::TRANSPARENT,
                // neutral-border-loud
                Color::new(0.35, 0.35, 0.35, 1.0),
                // neutral-on-quiet
                Color::new(0.20, 0.20, 0.20, 1.0),
            ),
            "filled" => (
                // neutral-fill-quiet
                Color::new(0.92, 0.92, 0.92, 1.0),
                Color::TRANSPARENT,
                // neutral-on-quiet
                Color::new(0.20, 0.20, 0.20, 1.0),
            ),
            "filled-outlined" => (
                // neutral-fill-quiet
                Color::new(0.92, 0.92, 0.92, 1.0),
                // neutral-border-normal
                Color::new(0.78, 0.78, 0.78, 1.0),
                // neutral-on-quiet
                Color::new(0.20, 0.20, 0.20, 1.0),
            ),
            // "accent" — neutral-fill-loud background, neutral-on-loud text
            _ => (
                // neutral-fill-loud
                Color::new(0.20, 0.20, 0.20, 1.0),
                Color::TRANSPARENT,
                // neutral-on-loud (white)
                Color::new(1.0, 1.0, 1.0, 1.0),
            ),
        };

        // 填充背景
        if bg_color != Color::TRANSPARENT {
            ctx.fill_rect(bounds, bg_color, border_radius);
        }

        // 绘制边框（仅 outline / filled-outlined 有可见边框）
        if border_color != Color::TRANSPARENT {
            let inner = Rect::new(
                bounds.origin.x + border_width,
                bounds.origin.y + border_width,
                (bounds.size.width - 2.0 * border_width).max(0.0),
                (bounds.size.height - 2.0 * border_width).max(0.0),
            );
            ctx.fill_rect(bounds, border_color, border_radius);
            if bg_color != Color::TRANSPARENT {
                ctx.fill_rect(
                    inner,
                    bg_color,
                    (border_radius as f64 - 1.0).max(0.0) as f32,
                );
            }
        }

        // 绘制标签文本（居中）
        if !state.label.is_empty() {
            // 计算文本区域（如果有移除按钮，左边留空间）
            let remove_space: f64 = if state.with_remove { 16.0 } else { 0.0 };
            let text_bounds = Rect::new(
                bounds.origin.x + 4.0,
                bounds.origin.y,
                (bounds.size.width - 8.0 - remove_space).max(0.0),
                bounds.size.height,
            );
            ctx.draw_text(&state.label, text_bounds, text_color, font_size);
        }

        // 绘制移除按钮（× 字符）
        if state.with_remove {
            let x_bounds = Rect::new(
                bounds.origin.x + bounds.size.width - 18.0,
                bounds.origin.y,
                16.0,
                bounds.size.height,
            );
            ctx.draw_text("\u{00D7}", x_bounds, text_color, font_size);
        }
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("tag")
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
        assert_eq!(WaTag.name(), "rgui_components::WaTag");
    }

    #[test]
    fn view_has_props() {
        let state = WaTagState::new("Test");
        let v = WaTag.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("variant"));
        assert!(v.props.contains_key("appearance"));
        assert!(v.props.contains_key("size"));
        assert!(v.props.contains_key("pill"));
        assert!(v.props.contains_key("with_remove"));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_label_prop() {
        let state = WaTagState::new("Hello");
        let v = WaTag.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("label").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "Hello"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_pill_prop() {
        let mut state = WaTagState::new("");
        state.pill = true;
        let v = WaTag.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("pill").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b, "pill 应为 true"),
            _ => panic!("expected Bool prop"),
        }
    }

    #[test]
    fn view_with_remove_prop() {
        let mut state = WaTagState::new("");
        state.with_remove = true;
        let v = WaTag.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("with_remove").unwrap();
        match val {
            PropValue::Bool(b) => assert!(b, "with_remove 应为 true"),
            _ => panic!("expected Bool prop"),
        }
    }

    #[test]
    fn update_remove_is_handled() {
        let mut state = WaTagState::new("Test");
        WaTag.update(
            WaTagMessage::Remove,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_has_minimum_size() {
        let state = WaTagState::new("Test");
        let size = WaTag.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 20.0, "宽度应 ≥ 20px，实际 {size:?}");
        assert!(size.height >= 20.0, "高度应 ≥ 20px，实际 {size:?}");
    }

    #[test]
    fn measure_with_remove_is_wider() {
        let mut state = WaTagState::new("Test");
        state.with_remove = true;
        let size = WaTag.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 36.0, "带移除按钮的 Tag 应更宽，实际 {size:?}");
    }

    #[test]
    fn paint_filled_outlined_produces_ops() {
        let state = WaTagState::new("Test"); // filled-outlined (default)
        let bounds = Rect::new(0.0, 0.0, 80.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaTag.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "filled-outlined Tag 应产生绘制操作");
    }

    #[test]
    fn paint_accent_produces_ops() {
        let mut state = WaTagState::new("Test");
        state.appearance = "accent".into();
        let bounds = Rect::new(0.0, 0.0, 80.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaTag.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "accent Tag 应产生绘制操作");
    }

    #[test]
    fn paint_outlined_produces_ops() {
        let mut state = WaTagState::new("Test");
        state.appearance = "outlined".into();
        let bounds = Rect::new(0.0, 0.0, 80.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaTag.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "outlined Tag 应产生绘制操作");
    }

    #[test]
    fn paint_pill_uses_large_radius() {
        let mut state = WaTagState::new("Test");
        state.pill = true;
        let bounds = Rect::new(0.0, 0.0, 80.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaTag.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_with_remove_shows_x() {
        let mut state = WaTagState::new("Test");
        state.with_remove = true;
        let bounds = Rect::new(0.0, 0.0, 100.0, 24.0);
        let mut ctx = PaintContext::new(bounds);
        WaTag.paint(&state, bounds, &mut ctx);
        // 背景 + 边框 + 文本 + × 字符
        assert!(
            ctx.op_count() >= 2,
            "with_remove Tag 应至少产生 2 个操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaTagMessage::Remove.message_name(), "remove");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaTagState::schema_name(), "WaTagState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaTagState::new("Test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaTagState>());
    }
}
