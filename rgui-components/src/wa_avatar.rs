/// Translated from Web Awesome wa-avatar
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

/// Web Awesome wa-avatar 组件状态。
///
/// 头像组件，用于展示人物或对象的图片、首字母或图标。
/// 当前阶段不加载实际图片，使用首字母或图标 fallback。
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaAvatarState {
    /// 头像图片 URL（当前阶段仅保留字段，不实际加载图片）
    pub image: String,
    /// 无障碍标签，描述头像给辅助设备
    pub label: String,
    /// 首字母 fallback（推荐 1-2 个字符），当无图片时显示
    pub initials: String,
    /// 形状：circle（默认）| square | rounded
    pub shape: String,
    /// 图片加载错误标记
    pub has_error: bool,
}

impl Default for WaAvatarState {
    fn default() -> Self {
        Self {
            image: String::new(),
            label: String::new(),
            initials: String::new(),
            shape: "circle".into(),
            has_error: false,
        }
    }
}

impl WaAvatarState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaAvatarMessage {
    /// 图片加载出错（wa-error 事件映射）
    Error,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaAvatar;

impl WidgetSpec for WaAvatar {
    type State = WaAvatarState;
    type Message = WaAvatarMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaAvatar"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaAvatar")
            .prop("image", PropValue::str(state.image.as_str()))
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("initials", PropValue::str(state.initials.as_str()))
            .prop("shape", PropValue::str(state.shape.as_str()))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaAvatarMessage::Error => {
                state.has_error = true;
            },
        }
    }

    fn measure(&self, _state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        // Avatar 默认尺寸：3rem = 48px（WA 的 --size 变量）
        let default: f64 = 48.0;
        let w = default.clamp(c.min_width, c.max_width);
        let h = default.clamp(c.min_height, c.max_height);
        Size::new(w, h)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 背景色：neutral-fill-normal（浅灰色背景）
        let bg_color = Color::new(0.85, 0.85, 0.85, 1.0);
        // 文字色：neutral-on-normal（深色文字）
        let text_color = Color::new(0.1, 0.1, 0.1, 1.0);

        // 圆角半径根据形状决定
        let border_radius: f32 = match state.shape.as_str() {
            "square" => 0.0,
            "rounded" => 8.0,
            // "circle" (default) — 50% 圆角
            _ => {
                let min_dim = bounds.size.width.min(bounds.size.height);
                (min_dim / 2.0) as f32
            },
        };

        // 填充背景
        ctx.fill_rect(bounds, bg_color, border_radius);

        // 绘制文字内容
        let text: String;
        if !state.initials.is_empty() {
            // 取前 2 个字符并转为大写
            text = state
                .initials
                .chars()
                .take(2)
                .collect::<String>()
                .to_uppercase();
        } else {
            // fallback：显示人物图标（Unicode）
            text = "\u{1F464}".to_string(); // 👤
        }

        // 字体大小：height * 0.4（对齐 WA CSS calc(var(--size) * 0.4)）
        let font_size: f32 = (bounds.size.height * 0.4) as f32;
        // 近似字符宽度（大写字母平均宽度约为 font_size * 0.5）
        let char_count = text.chars().count().max(1) as f64;
        let text_width = (font_size as f64) * 0.6 * char_count;
        let text_height = font_size as f64;

        let text_bounds = Rect::new(
            bounds.origin.x + (bounds.size.width - text_width) / 2.0,
            bounds.origin.y + (bounds.size.height - text_height) / 2.0,
            text_width,
            text_height,
        );
        ctx.draw_text(&text, text_bounds, text_color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label = if state.label.is_empty() {
            "Avatar"
        } else {
            &state.label
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
        assert_eq!(WaAvatar.name(), "rgui_components::WaAvatar");
    }

    #[test]
    fn view_has_widget_type() {
        let state = WaAvatarState::new();
        let v = WaAvatar.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaAvatar");
    }

    #[test]
    fn view_has_props() {
        let mut state = WaAvatarState::new();
        state.image = "avatar.png".into();
        state.initials = "AB".into();
        state.shape = "rounded".into();
        state.label = "User Avatar".into();

        let v = WaAvatar.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("image"));
        assert!(v.props.contains_key("label"));
        assert!(v.props.contains_key("initials"));
        assert!(v.props.contains_key("shape"));
    }

    #[test]
    fn view_shape_prop() {
        let mut state = WaAvatarState::new();
        state.shape = "square".into();
        let v = WaAvatar.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("shape").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "square"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_error_sets_has_error() {
        let mut state = WaAvatarState::new();
        assert!(!state.has_error);
        WaAvatar.update(
            WaAvatarMessage::Error,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.has_error, "Error 消息应设置 has_error = true");
    }

    #[test]
    fn measure_has_default_size() {
        let state = WaAvatarState::new();
        let size = WaAvatar.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 48.0, "宽度应 ≥ 48px，实际 {size:?}");
        assert!(size.height >= 48.0, "高度应 ≥ 48px，实际 {size:?}");
    }

    #[test]
    fn measure_respects_max_constraints() {
        let state = WaAvatarState::new();
        let size = WaAvatar.measure(
            &state,
            BoxConstraints::new(0.0, 32.0, 0.0, 32.0),
            &MeasureContext::default(),
        );
        assert!(
            size.width <= 32.0,
            "宽度应受 max 约束 ≤ 32px，实际 {size:?}"
        );
        assert!(
            size.height <= 32.0,
            "高度应受 max 约束 ≤ 32px，实际 {size:?}"
        );
    }

    #[test]
    fn paint_produces_ops() {
        let state = WaAvatarState::new();
        let bounds = Rect::new(0.0, 0.0, 48.0, 48.0);
        let mut ctx = PaintContext::new(bounds);
        WaAvatar.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1, "Avatar 应产生绘制操作");
    }

    #[test]
    fn paint_with_initials() {
        let mut state = WaAvatarState::new();
        state.initials = "ab".into();
        let bounds = Rect::new(0.0, 0.0, 48.0, 48.0);
        let mut ctx = PaintContext::new(bounds);
        WaAvatar.paint(&state, bounds, &mut ctx);
        // 不 panic 即通过；至少产生背景填充 + 文字绘制
        assert!(
            ctx.op_count() >= 2,
            "有 initials 时应至少有背景 + 文字，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_square_shape() {
        let mut state = WaAvatarState::new();
        state.shape = "square".into();
        let bounds = Rect::new(0.0, 0.0, 48.0, 48.0);
        let mut ctx = PaintContext::new(bounds);
        WaAvatar.paint(&state, bounds, &mut ctx);
        // 不 panic = border_radius=0 可正常工作
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn paint_rounded_shape() {
        let mut state = WaAvatarState::new();
        state.shape = "rounded".into();
        let bounds = Rect::new(0.0, 0.0, 48.0, 48.0);
        let mut ctx = PaintContext::new(bounds);
        WaAvatar.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 1);
    }

    #[test]
    fn accessibility_default_label() {
        let state = WaAvatarState::new();
        let node = WaAvatar.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(
            node.label.as_deref(),
            Some("Avatar"),
            "默认无障碍标签应为 Avatar"
        );
    }

    #[test]
    fn accessibility_custom_label() {
        let mut state = WaAvatarState::new();
        state.label = "John Doe".into();
        let node = WaAvatar.accessibility(&state, &AccessContext::new(Rect::ZERO));
        assert_eq!(
            node.label.as_deref(),
            Some("John Doe"),
            "应使用自定义无障碍标签"
        );
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaAvatarMessage::Error.message_name(), "error");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaAvatarState::schema_name(), "WaAvatarState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaAvatarState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaAvatarState>());
    }
}
