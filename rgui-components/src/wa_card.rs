/// Translated from Web Awesome wa-card
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

/// Web Awesome wa-card 组件状态。
///
/// Card 是一种装饰容器，将相关内容组织在带边框和背景的卡片中。
/// 支持 header/footer/media 命名 slot（通过 children 遍历判断）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaCardState {
    /// 视觉外观：accent | filled | outlined | filled-outlined | plain
    pub appearance: String,
    /// 方向：horizontal | vertical
    pub orientation: String,
}

impl WaCardState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            appearance: "outlined".into(),
            orientation: "vertical".into(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaCardMessage {
    /// 占位消息——Card 无交互事件
    NoOp,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaCard;

impl WidgetSpec for WaCard {
    type State = WaCardState;
    type Message = WaCardMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaCard"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaCard")
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("orientation", PropValue::str(state.orientation.as_str()))
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaCardMessage::NoOp => {},
        }
    }

    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        // Card 是容器，尺寸由 Taffy 根据子节点和约束计算
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let border_radius: f32 = 8.0;
        let border_width: f64 = 1.0;

        // 根据 appearance 确定背景色和边框色
        let (bg_color, border_color) = match state.appearance.as_str() {
            "plain" => (Color::TRANSPARENT, Color::TRANSPARENT),
            "filled" => (
                // neutral-fill-quiet: 浅灰背景
                Color::new(0.94, 0.94, 0.95, 1.0),
                Color::TRANSPARENT,
            ),
            "filled-outlined" => (
                Color::new(0.94, 0.94, 0.95, 1.0),
                // surface-border
                Color::new(0.82, 0.82, 0.84, 1.0),
            ),
            "accent" => (
                // neutral-fill-loud: 深色背景
                Color::new(0.18, 0.18, 0.22, 1.0),
                Color::TRANSPARENT,
            ),
            // "outlined"（默认）
            _ => (
                Color::new(0.98, 0.98, 0.99, 1.0), // surface-default
                Color::new(0.82, 0.82, 0.84, 1.0), // surface-border
            ),
        };

        // 绘制背景矩形
        if bg_color.a > 0.0 {
            ctx.fill_rect(bounds, bg_color, border_radius);
        }

        // 绘制边框（通过略微缩小的矩形描边模拟）
        if border_color.a > 0.0 && bounds.size.width > 0.0 && bounds.size.height > 0.0 {
            // 使用 fill_rect 绘制边框：先绘制一个与外框相同大小的透明圆角矩形，
            // 然后在内侧绘制一个稍小的背景色矩形覆盖，形成边框效果
            // 简化方案：直接绘制四条边
            let x = bounds.origin.x;
            let y = bounds.origin.y;
            let w = bounds.size.width;
            let h = bounds.size.height;

            // 上边框
            ctx.fill_rect(Rect::new(x, y, w, border_width.min(h)), border_color, 0.0);
            // 下边框
            ctx.fill_rect(
                Rect::new(x, y + h - border_width, w, border_width.min(h)),
                border_color,
                0.0,
            );
            // 左边框
            ctx.fill_rect(
                Rect::new(
                    x,
                    y + border_width,
                    border_width.min(w),
                    h - 2.0 * border_width,
                ),
                border_color,
                0.0,
            );
            // 右边框
            ctx.fill_rect(
                Rect::new(
                    x + w - border_width,
                    y + border_width,
                    border_width.min(w),
                    h - 2.0 * border_width,
                ),
                border_color,
                0.0,
            );
        }
    }

    fn accessibility(&self, _state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label("card")
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
        assert_eq!(WaCard.name(), "rgui_components::WaCard");
    }

    #[test]
    fn view_has_appearance() {
        let state = WaCardState::new();
        let v = WaCard.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("appearance"));
    }

    #[test]
    fn view_default_outlined() {
        let state = WaCardState::new();
        let v = WaCard.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        let val = v.props.get("appearance").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "outlined"),
            _ => panic!("expected Str prop"),
        }
    }

    #[test]
    fn update_noop_is_handled() {
        let mut state = WaCardState::new();
        WaCard.update(
            WaCardMessage::NoOp,
            &mut state,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn measure_returns_zero_delegating_to_layout() {
        let state = WaCardState::new();
        let size = WaCard.measure(
            &state,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO, "Card 容器委托 Taffy 计算尺寸");
    }

    #[test]
    fn paint_outlined_produces_ops() {
        let state = WaCardState::new(); // default "outlined"
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaCard.paint(&state, bounds, &mut ctx);
        // outlined: 背景 + 四条边框 = 至少 5 个操作
        assert!(
            ctx.op_count() >= 5,
            "outlined Card 应产生背景+边框绘制操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_plain_produces_no_ops() {
        let state = WaCardState {
            appearance: "plain".into(),
            orientation: "vertical".into(),
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaCard.paint(&state, bounds, &mut ctx);
        // plain: 透明背景 + 透明边框 = 0 操作
        assert_eq!(ctx.op_count(), 0, "plain Card 不产生绘制操作");
    }

    #[test]
    fn paint_filled_produces_background_only() {
        let state = WaCardState {
            appearance: "filled".into(),
            orientation: "vertical".into(),
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaCard.paint(&state, bounds, &mut ctx);
        // filled: 背景 + 无边框 = 1 操作
        assert_eq!(ctx.op_count(), 1, "filled Card 仅绘制背景");
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaCardMessage::NoOp.message_name(), "no_op");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaCardState::schema_name(), "WaCardState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaCardState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaCardState>());
    }
}
