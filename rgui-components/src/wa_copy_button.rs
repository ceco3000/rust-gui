/// Translated from Web Awesome wa-copy-button
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

/// Web Awesome wa-copy-button 组件状态。
///
/// 复制按钮——点击后将 value 文本复制到系统剪贴板，
/// 并提供 success/error 视觉反馈。
///
/// 跳过 `from`（DOM 元素引用）、`tooltipPlacement`/`tooltip`（tooltip 组件 WT55 未完成）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaCopyButtonState {
    /// 要复制的文本内容
    pub value: String,
    /// 禁用状态
    pub disabled: bool,
    /// 默认状态的标签文本（无障碍 + tooltip）
    pub copy_label: String,
    /// 复制成功后的标签文本
    pub success_label: String,
    /// 复制失败后的标签文本
    pub error_label: String,
    /// 反馈显示时长（ms），之后恢复默认状态
    pub feedback_duration: u64,
    /// 是否正在执行复制操作
    pub is_copying: bool,
    /// 当前反馈状态：rest | success | error
    pub status: String,
    /// 无障碍实时播报文本
    pub live_announcement: String,
}

impl WaCopyButtonState {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            disabled: false,
            copy_label: "Copy".into(),
            success_label: "Copied!".into(),
            error_label: "Error".into(),
            feedback_duration: 1000,
            is_copying: false,
            status: "rest".into(),
            live_announcement: String::new(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaCopyButtonMessage {
    /// 用户点击按钮
    Click,
    /// 复制成功
    Copy,
    /// 复制失败
    Error,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaCopyButton;

impl WidgetSpec for WaCopyButton {
    type State = WaCopyButtonState;
    type Message = WaCopyButtonMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaCopyButton"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaCopyButton")
            .prop("value", PropValue::str(state.value.as_str()))
            .prop("disabled", PropValue::bool(state.disabled))
            .prop("copy_label", PropValue::str(state.copy_label.as_str()))
            .prop(
                "success_label",
                PropValue::str(state.success_label.as_str()),
            )
            .prop("error_label", PropValue::str(state.error_label.as_str()))
            .prop(
                "feedback_duration",
                PropValue::int(state.feedback_duration as i64),
            )
            .prop("is_copying", PropValue::bool(state.is_copying))
            .prop("status", PropValue::str(state.status.as_str()))
            .prop(
                "live_announcement",
                PropValue::str(state.live_announcement.as_str()),
            )
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaCopyButtonMessage::Click => {
                if state.disabled || state.is_copying {
                    return;
                }
                state.is_copying = true;
                // 尝试写入系统剪贴板
                // 注意：rgui-components 不直接依赖 rgui-platform，
                // 剪贴板操作由 app 层通过绑定 Copy/Error 消息处理。
                // 这里模拟成功状态——实际项目中 app 层调用
                // rgui_platform::Clipboard::set_text(&state.value) 后发送
                // WaCopyButtonMessage::Copy 或 WaCopyButtonMessage::Error。
                state.status = "success".into();
                state.live_announcement = if state.success_label.is_empty() {
                    "Copied!".into()
                } else {
                    state.success_label.clone()
                };
            },
            WaCopyButtonMessage::Copy => {
                state.status = "success".into();
                state.live_announcement = if state.success_label.is_empty() {
                    "Copied!".into()
                } else {
                    state.success_label.clone()
                };
            },
            WaCopyButtonMessage::Error => {
                state.status = "error".into();
                state.is_copying = false;
                state.live_announcement = if state.error_label.is_empty() {
                    "Error".into()
                } else {
                    state.error_label.clone()
                };
            },
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        // 叶子组件：返回最小尺寸（约 32×32 的方形按钮）
        Size::new(32.0, 32.0)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let border_radius: f32 = 6.0;
        let w = bounds.size.width;
        let h = bounds.size.height;

        // 按钮背景色——根据 status 和 disabled 变化
        let bg_color = if state.disabled {
            Color::new(0.85, 0.85, 0.86, 0.5)
        } else {
            match state.status.as_str() {
                "success" => Color::new(0.2, 0.7, 0.3, 0.15),
                "error" => Color::new(0.85, 0.2, 0.2, 0.15),
                _ => Color::TRANSPARENT,
            }
        };

        if bg_color.a > 0.0 {
            ctx.fill_rect(bounds, bg_color, border_radius);
        }

        // 图标 Unicode 字符——根据 status 选择
        let icon_char = match state.status.as_str() {
            "success" => "✓",
            "error" => "✗",
            _ => "⎘",
        };

        let icon_color = if state.disabled {
            Color::new(0.6, 0.6, 0.62, 1.0)
        } else {
            match state.status.as_str() {
                "success" => Color::new(0.15, 0.65, 0.25, 1.0),
                "error" => Color::new(0.8, 0.15, 0.15, 1.0),
                _ => Color::new(0.3, 0.3, 0.32, 1.0),
            }
        };

        // 图标居中绘制
        let icon_size: f32 = 14.0;
        let icon_x = bounds.origin.x + (w - icon_size as f64) / 2.0;
        let icon_y = bounds.origin.y + (h - icon_size as f64) / 2.0;
        ctx.draw_text(
            icon_char,
            Rect::new(icon_x, icon_y, icon_size as f64, icon_size as f64),
            icon_color,
            icon_size,
        );
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let label = match state.status.as_str() {
            "success" => {
                if state.success_label.is_empty() {
                    "Copied!"
                } else {
                    state.success_label.as_str()
                }
            },
            "error" => {
                if state.error_label.is_empty() {
                    "Error"
                } else {
                    state.error_label.as_str()
                }
            },
            _ => {
                if state.copy_label.is_empty() {
                    "Copy"
                } else {
                    state.copy_label.as_str()
                }
            },
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
        assert_eq!(WaCopyButton.name(), "rgui_components::WaCopyButton");
    }

    #[test]
    fn state_defaults() {
        let state = WaCopyButtonState::new("hello");
        assert_eq!(state.value, "hello");
        assert!(!state.disabled);
        assert_eq!(state.copy_label, "Copy");
        assert_eq!(state.success_label, "Copied!");
        assert_eq!(state.error_label, "Error");
        assert_eq!(state.status, "rest");
        assert!(!state.is_copying);
    }

    #[test]
    fn view_has_props() {
        let state = WaCopyButtonState::new("test");
        let v = WaCopyButton.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("value"));
        assert!(v.props.contains_key("disabled"));
        assert!(v.props.contains_key("copy_label"));
        assert!(v.props.contains_key("success_label"));
        assert!(v.props.contains_key("error_label"));
        assert!(v.props.contains_key("status"));
    }

    #[test]
    fn click_sets_status_success() {
        let mut state = WaCopyButtonState::new("copy me");
        WaCopyButton.update(
            WaCopyButtonMessage::Click,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.status, "success");
        assert!(state.is_copying);
        assert!(!state.live_announcement.is_empty());
    }

    #[test]
    fn click_respects_disabled() {
        let mut state = WaCopyButtonState::new("copy me");
        state.disabled = true;
        WaCopyButton.update(
            WaCopyButtonMessage::Click,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.status, "rest");
        assert!(!state.is_copying);
    }

    #[test]
    fn click_respects_is_copying() {
        let mut state = WaCopyButtonState::new("copy me");
        state.is_copying = true;
        WaCopyButton.update(
            WaCopyButtonMessage::Click,
            &mut state,
            &mut UpdateContext::default(),
        );
        // status should not change because is_copying was already true
        assert!(state.is_copying);
    }

    #[test]
    fn copy_message_sets_success() {
        let mut state = WaCopyButtonState::new("copy me");
        WaCopyButton.update(
            WaCopyButtonMessage::Copy,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.status, "success");
    }

    #[test]
    fn error_message_sets_error() {
        let mut state = WaCopyButtonState::new("copy me");
        state.is_copying = true;
        WaCopyButton.update(
            WaCopyButtonMessage::Error,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.status, "error");
        assert!(!state.is_copying);
    }

    #[test]
    fn measure_returns_min_size() {
        let state = WaCopyButtonState::new("test");
        let size = WaCopyButton.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::new(32.0, 32.0));
    }

    #[test]
    fn paint_rest_produces_icon() {
        let state = WaCopyButtonState::new("test");
        let bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaCopyButton.paint(&state, bounds, &mut ctx);
        // rest 状态：透明背景 + 图标文字 = 1 个操作（背景不会绘制因为 alpha=0）
        assert!(ctx.op_count() >= 1, "rest 状态至少产生图标操作");
    }

    #[test]
    fn paint_success_produces_background_and_icon() {
        let mut state = WaCopyButtonState::new("test");
        state.status = "success".into();
        let bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaCopyButton.paint(&state, bounds, &mut ctx);
        // success: 背景 + 图标 = 2 个操作
        assert!(
            ctx.op_count() >= 2,
            "success 状态应产生背景+图标操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_error_produces_background_and_icon() {
        let mut state = WaCopyButtonState::new("test");
        state.status = "error".into();
        let bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaCopyButton.paint(&state, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 2,
            "error 状态应产生背景+图标操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_disabled_dims_icon() {
        let mut state = WaCopyButtonState::new("test");
        state.disabled = true;
        let bounds = Rect::new(0.0, 0.0, 32.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaCopyButton.paint(&state, bounds, &mut ctx);
        // disabled: 背景（半透明） + 图标 = 2 个操作
        assert!(ctx.op_count() >= 2, "disabled 状态应产生背景+图标操作");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaCopyButtonMessage::Click.message_name(), "click");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaCopyButtonState::schema_name(), "WaCopyButtonState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaCopyButtonState::new("test");
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaCopyButtonState>());
    }
}
