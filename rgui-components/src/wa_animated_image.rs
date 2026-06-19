/// Translated from Web Awesome wa-animated-image
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

/// Web Awesome wa-animated-image 组件状态。
///
/// 动画图片组件，显示 GIF/WEBP 并提供播放/暂停控制。
/// 阶段 0：渲染占位矩形 + 播放/暂停 Unicode 图标 + alt 文本。
/// 后续阶段将支持实际图片加载与首帧冻结。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaAnimatedImageState {
    /// 图片路径（阶段 0 保留字段，不实际加载）
    pub src: String,
    /// 辅助设备描述文本
    pub alt: String,
    /// 是否正在播放
    pub play: bool,
    /// 图片是否已加载（阶段 0 始终为 true）
    pub is_loaded: bool,
}

impl WaAnimatedImageState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            src: String::new(),
            alt: String::new(),
            play: false,
            is_loaded: true,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaAnimatedImageMessage {
    /// 图片加载成功（wa-load 事件映射）
    Load,
    /// 图片加载失败（wa-error 事件映射）
    Error,
    /// 切换播放/暂停（点击或键盘触发）
    TogglePlay,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaAnimatedImage;

impl WidgetSpec for WaAnimatedImage {
    type State = WaAnimatedImageState;
    type Message = WaAnimatedImageMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaAnimatedImage"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaAnimatedImage")
            .prop("src", PropValue::str(state.src.as_str()))
            .prop("alt", PropValue::str(state.alt.as_str()))
            .prop("play", PropValue::bool(state.play))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaAnimatedImageMessage::Load => {
                state.is_loaded = true;
            },
            WaAnimatedImageMessage::Error => {
                // 阶段 0：无特殊处理
            },
            WaAnimatedImageMessage::TogglePlay => {
                state.play = !state.play;
            },
        }
    }

    /// 默认尺寸 320×240，受约束限制。
    fn measure(&self, _state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let default_w: f64 = 320.0;
        let default_h: f64 = 240.0;
        let w = default_w.clamp(c.min_width, c.max_width);
        let h = default_h.clamp(c.min_height, c.max_height);
        Size::new(w, h)
    }

    /// 阶段 0 渲染：深色占位背景 + 居中控制圆 + 播放/暂停图标 + alt 文本。
    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let w = bounds.size.width;
        let h = bounds.size.height;

        // 占位背景（深灰色，模拟图片区域）
        let bg_color = Color::new(0.15, 0.15, 0.15, 1.0);
        let border_radius: f32 = 4.0;
        ctx.fill_rect(bounds, bg_color, border_radius);

        // 控制圆：居中，尺寸为 min(w,h) * 0.2
        let ctrl_size = (w.min(h) * 0.2).max(30.0);
        let ctrl_radius: f32 = (ctrl_size / 2.0) as f32;
        let ctrl_rect = Rect::new(
            bounds.origin.x + (w - ctrl_size) / 2.0,
            bounds.origin.y + (h - ctrl_size) / 2.0,
            ctrl_size,
            ctrl_size,
        );

        // 控制圆背景（半透明黑色）
        let ctrl_bg = Color::new(0.0, 0.0, 0.0, 0.5);
        ctx.fill_rect(ctrl_rect, ctrl_bg, ctrl_radius);

        // 播放/暂停图标
        let icon_char = if state.play { "\u{23F8}" } else { "\u{25B6}" }; // ⏸ / ▶
        let icon_color = Color::new(1.0, 1.0, 1.0, 1.0);
        let icon_font_size: f32 = (ctrl_size * 0.5) as f32;
        ctx.draw_text(icon_char, ctrl_rect, icon_color, icon_font_size);

        // alt 文本（右下角，小字）
        if !state.alt.is_empty() {
            let alt_color = Color::new(0.6, 0.6, 0.6, 1.0);
            let alt_font_size: f32 = (h * 0.06).max(10.0) as f32;
            let alt_y = bounds.origin.y + h - (alt_font_size as f64) - 4.0;
            let alt_rect = Rect::new(
                bounds.origin.x + 4.0,
                alt_y,
                w - 8.0,
                alt_font_size as f64 + 4.0,
            );
            ctx.draw_text(state.alt.as_str(), alt_rect, alt_color, alt_font_size);
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label = if state.alt.is_empty() {
            "Animated Image"
        } else {
            state.alt.as_str()
        };
        let role = if state.play {
            "Pause animation button"
        } else {
            "Play animation button"
        };
        let full_label = format!("{label} — {role}");
        AccessibilityNode::none().label(full_label)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let state = WaAnimatedImageState::new();
        assert_eq!(state.src, "");
        assert_eq!(state.alt, "");
        assert!(!state.play);
        assert!(state.is_loaded);
    }

    #[test]
    fn state_with_src_and_alt() {
        let state = WaAnimatedImageState {
            src: "animation.gif".into(),
            alt: "Loading animation".into(),
            play: true,
            is_loaded: true,
        };
        assert_eq!(state.src, "animation.gif");
        assert_eq!(state.alt, "Loading animation");
        assert!(state.play);
    }

    #[test]
    fn name_returns_correct_path() {
        assert_eq!(WaAnimatedImage.name(), "rgui_components::WaAnimatedImage");
    }

    #[test]
    fn message_toggle_play() {
        let mut state = WaAnimatedImageState::new();
        assert!(!state.play);
        WaAnimatedImage.update(
            WaAnimatedImageMessage::TogglePlay,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.play, "TogglePlay 应切换 play 为 true");
        WaAnimatedImage.update(
            WaAnimatedImageMessage::TogglePlay,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.play, "再次 TogglePlay 应切换回 false");
    }

    #[test]
    fn message_load() {
        let mut state = WaAnimatedImageState {
            is_loaded: false,
            ..WaAnimatedImageState::new()
        };
        WaAnimatedImage.update(
            WaAnimatedImageMessage::Load,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.is_loaded);
    }

    #[test]
    fn message_error_noop() {
        let mut state = WaAnimatedImageState::new();
        let before = state.clone();
        WaAnimatedImage.update(
            WaAnimatedImageMessage::Error,
            &mut state,
            &mut UpdateContext::default(),
        );
        // Error 在阶段 0 无副作用
        assert_eq!(state.play, before.play);
        assert_eq!(state.is_loaded, before.is_loaded);
    }

    #[test]
    fn measure_has_default_size() {
        let state = WaAnimatedImageState::new();
        let constraints = BoxConstraints::new(0.0, 800.0, 0.0, 600.0);
        let ctx = MeasureContext::default();
        let size = WaAnimatedImage.measure(&state, constraints, &ctx);
        assert!(
            (size.width - 320.0).abs() < 0.01,
            "默认宽度应为 320，实际 {size:?}"
        );
        assert!(
            (size.height - 240.0).abs() < 0.01,
            "默认高度应为 240，实际 {size:?}"
        );
    }

    #[test]
    fn measure_clamped_by_constraints() {
        let state = WaAnimatedImageState::new();
        let constraints = BoxConstraints::new(0.0, 100.0, 0.0, 80.0);
        let ctx = MeasureContext::default();
        let size = WaAnimatedImage.measure(&state, constraints, &ctx);
        assert!(
            size.width <= 100.0,
            "宽度应受 max 约束 ≤ 100，实际 {size:?}"
        );
        assert!(size.height <= 80.0, "高度应受 max 约束 ≤ 80，实际 {size:?}");
    }

    #[test]
    fn measure_zero_constraints_returns_zero() {
        let state = WaAnimatedImageState::new();
        let constraints = BoxConstraints::new(0.0, 0.0, 0.0, 0.0);
        let ctx = MeasureContext::default();
        let size = WaAnimatedImage.measure(&state, constraints, &ctx);
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_produces_ops() {
        let state = WaAnimatedImageState::new();
        let bounds = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ctx = PaintContext::new(bounds);
        WaAnimatedImage.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(
            ops.len() >= 2,
            "应产生至少 2 个绘制操作（背景 + 控制圆），实际 {}",
            ops.len()
        );
    }

    #[test]
    fn paint_playing_shows_pause_icon() {
        let state = WaAnimatedImageState {
            play: true,
            ..WaAnimatedImageState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ctx = PaintContext::new(bounds);
        WaAnimatedImage.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.len() >= 2);
    }

    #[test]
    fn paint_with_alt_text() {
        let state = WaAnimatedImageState {
            alt: "Test animation".into(),
            ..WaAnimatedImageState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ctx = PaintContext::new(bounds);
        WaAnimatedImage.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(
            ops.len() >= 3,
            "有 alt 文本时应至少有背景+控制圆+文本，实际 {}",
            ops.len()
        );
    }

    #[test]
    fn paint_zero_bounds_no_ops() {
        let state = WaAnimatedImageState::new();
        let bounds = Rect::ZERO;
        let mut ctx = PaintContext::new(bounds);
        WaAnimatedImage.paint(&state, bounds, &mut ctx);
        let ops = ctx.into_operations();
        assert!(ops.is_empty(), "零尺寸不应产生绘制操作");
    }

    #[test]
    fn view_has_props() {
        let state = WaAnimatedImageState {
            src: "image.gif".into(),
            alt: "Animation".into(),
            play: true,
            is_loaded: true,
        };
        let v = WaAnimatedImage.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.widget_type, "rgui_components::WaAnimatedImage");
        assert!(v.props.contains_key("src"));
        assert!(v.props.contains_key("alt"));
        assert!(v.props.contains_key("play"));
    }

    #[test]
    fn view_src_prop() {
        let mut state = WaAnimatedImageState::new();
        state.src = "test.gif".into();
        let v = WaAnimatedImage.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("src").unwrap() {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "test.gif"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn view_play_prop() {
        let mut state = WaAnimatedImageState::new();
        state.play = true;
        let v = WaAnimatedImage.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("play").unwrap() {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop"),
        }
    }

    #[test]
    fn view_play_false_prop() {
        let state = WaAnimatedImageState::new();
        let v = WaAnimatedImage.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        match v.props.get("play").unwrap() {
            PropValue::Bool(b) => assert!(!*b),
            _ => panic!("expected Bool prop"),
        }
    }

    #[test]
    fn accessibility_with_alt() {
        let state = WaAnimatedImageState {
            alt: "Loading spinner".into(),
            play: false,
            ..WaAnimatedImageState::new()
        };
        let node = WaAnimatedImage.accessibility(&state, &AccessContext::new(Rect::ZERO));
        let label = node.label.as_deref().unwrap();
        assert!(label.contains("Loading spinner"));
        assert!(label.contains("Play"));
    }

    #[test]
    fn accessibility_without_alt() {
        let state = WaAnimatedImageState::new();
        let node = WaAnimatedImage.accessibility(&state, &AccessContext::new(Rect::ZERO));
        let label = node.label.as_deref().unwrap();
        assert!(label.contains("Animated Image"));
    }

    #[test]
    fn accessibility_playing() {
        let state = WaAnimatedImageState {
            alt: "Demo".into(),
            play: true,
            ..WaAnimatedImageState::new()
        };
        let node = WaAnimatedImage.accessibility(&state, &AccessContext::new(Rect::ZERO));
        let label = node.label.as_deref().unwrap();
        assert!(label.contains("Pause"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(
            WaAnimatedImageMessage::TogglePlay.message_name(),
            "toggle_play"
        );
        assert_eq!(WaAnimatedImageMessage::Load.message_name(), "load");
        assert_eq!(WaAnimatedImageMessage::Error.message_name(), "error");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaAnimatedImageState::schema_name(), "WaAnimatedImageState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaAnimatedImageState::new();
        let any = state.as_any();
        assert_eq!(
            any.type_id(),
            std::any::TypeId::of::<WaAnimatedImageState>()
        );
    }
}
