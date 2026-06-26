// MIT License
//
// Copyright (c) 2025 rgui contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! WaBadge 组件——状态/计数/标签展示组件。
//!
//! 从 Web Awesome `<wa-badge>` (MIT) 手工翻译为 rgui Tier 1 WidgetSpec。
//!
//! ## 阶段 0 简化
//!
//! - `variant` prop 存储但不参与颜色选择（所有 variant 使用相同品牌色）
//! - `attention` 动画跳过（仅存储）
//! - `start`/`end` named slot 跳过（仅支持通过 label prop 传递默认 slot）
//! - 边框用 4 条 `fill_rect` 模拟（框架无 `stroke_rect` 原语）

use rgui_core::a11y::AccessibilityNode;
use rgui_core::context::{AccessContext, MeasureContext, PaintContext, UpdateContext, ViewContext};
use rgui_core::geometry::{BoxConstraints, Rect, Size};
use rgui_core::traits::WidgetSpec;
use rgui_core::view::{Color, WidgetView};
use rgui_macros::{AppMessage, PersistState};

// ============================================================================
// WaBadgeState
// ============================================================================

/// WaBadge 持久状态。
///
/// 字段对应 WA 源的 4 个 `@property` + `label`（默认 slot 内容）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, PersistState)]
pub struct WaBadgeState {
    /// 默认 slot 文本内容。
    pub label: String,
    /// 语义变体（brand/neutral/success/warning/danger）。阶段 0 存储但不区分颜色。
    pub variant: String,
    /// 视觉外观（accent/filled/outlined/filled-outlined）。
    pub appearance: String,
    /// 全圆角模式。
    pub pill: bool,
    /// 动画类型（none/pulse/bounce）。阶段 0 仅存储不渲染。
    pub attention: String,
}

impl Default for WaBadgeState {
    fn default() -> Self {
        Self {
            label: String::new(),
            variant: String::from("brand"),
            appearance: String::from("accent"),
            pill: false,
            attention: String::from("none"),
        }
    }
}

// ============================================================================
// WaBadgeMessage
// ============================================================================

/// WaBadge 消息类型——零事件组件，使用 NoOp 占位变体。
#[derive(Debug, Clone, PartialEq, AppMessage)]
pub enum WaBadgeMessage {
    /// 占位消息（WA 源 Badge 无事件声明）。
    NoOp,
}

// ============================================================================
// WaBadge
// ============================================================================

/// WaBadge 组件——首个生产级 Tier 1 WidgetSpec 实现。
pub struct WaBadge;

impl WidgetSpec for WaBadge {
    type State = WaBadgeState;
    type Message = WaBadgeMessage;

    fn name(&self) -> &'static str {
        "WaBadge"
    }

    fn view(&self, state: &Self::State, _ctx: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("WaBadge")
            .prop("label", state.label.clone())
            .prop("variant", state.variant.clone())
            .prop("appearance", state.appearance.clone())
            .prop("pill", state.pill)
            .prop("attention", state.attention.clone())
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _ctx: &mut UpdateContext) {
        match msg {
            WaBadgeMessage::NoOp => {},
        }
    }

    fn measure(
        &self,
        state: &Self::State,
        constraints: BoxConstraints,
        _ctx: &MeasureContext,
    ) -> Size {
        // 空 label → Size::ZERO（L04）
        if state.label.is_empty() {
            return Size::ZERO;
        }

        // 零约束 → Size::ZERO（R4 边界：零约束短路）
        if constraints.max_width <= 0.0 && constraints.max_height <= 0.0 {
            return Size::ZERO;
        }

        const PAD_H: f64 = 10.0;
        const PAD_V: f64 = 6.0;
        const EM_HEIGHT: f64 = 1.448;
        const PAINT_FONT_RATIO: f64 = 0.44;
        const MAX_FONT_SIZE: f64 = 24.0;

        let font_size = constraints.max_height.min(MAX_FONT_SIZE) * PAINT_FONT_RATIO;
        let char_count = state.label.chars().count() as f64;
        let width = char_count * font_size * 0.6 + PAD_H * 2.0;
        let height = font_size * EM_HEIGHT + PAD_V * 2.0;

        Size::new(width, height)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        // 空 label → early return（L04）
        if state.label.is_empty() {
            return;
        }

        // 零尺寸 bounds → early return
        if bounds.size.is_empty() {
            return;
        }

        // 颜色选择（R2），未知 appearance 降级为 accent
        let (bg_color, border_color, text_color, has_bg, has_border) =
            match state.appearance.as_str() {
                "accent" => (
                    Color::new(0.09, 0.30, 0.90, 1.0),
                    Color::BLACK, // unused
                    Color::WHITE,
                    true,
                    false,
                ),
                "filled" => (
                    Color::new(0.85, 0.91, 1.0, 1.0),
                    Color::BLACK, // unused
                    Color::new(0.05, 0.10, 0.40, 1.0),
                    true,
                    false,
                ),
                "outlined" => (
                    Color::BLACK, // unused, no bg
                    Color::new(0.09, 0.30, 0.90, 1.0),
                    Color::new(0.09, 0.30, 0.90, 1.0),
                    false,
                    true,
                ),
                "filled-outlined" => (
                    Color::new(0.85, 0.91, 1.0, 1.0),
                    Color::new(0.60, 0.75, 0.95, 1.0),
                    Color::new(0.05, 0.10, 0.40, 1.0),
                    true,
                    true,
                ),
                // 未知 appearance → 降级为 accent（R2 异常路径）
                _ => (
                    Color::new(0.09, 0.30, 0.90, 1.0),
                    Color::BLACK,
                    Color::WHITE,
                    true,
                    false,
                ),
            };

        // 圆角半径（R3）
        let radius = if state.pill {
            (bounds.size.height / 2.0) as f32
        } else {
            4.0_f32
        };

        // 背景填充
        if has_bg {
            ctx.fill_rect(bounds, bg_color, radius);
        }

        // 边框：4 条 fill_rect 模拟 stroke（L08）
        if has_border {
            let x = bounds.origin.x;
            let y = bounds.origin.y;
            let w = bounds.size.width;
            let h = bounds.size.height;

            if state.pill {
                // pill 模式：边框内缩 radius，跟随圆角背景曲线
                let r = radius as f64;
                let tb_w = (w - 2.0 * r).max(0.0);
                let lr_h = (h - 2.0 * r).max(0.0);

                if tb_w > 0.0 {
                    // 上边框：内缩 r
                    ctx.fill_rect(Rect::new(x + r, y, tb_w, 1.0), border_color, 0.0);
                    // 下边框：内缩 r
                    ctx.fill_rect(Rect::new(x + r, y + h - 1.0, tb_w, 1.0), border_color, 0.0);
                }
                if lr_h > 0.0 {
                    // 左边框：高度内缩 r
                    ctx.fill_rect(Rect::new(x, y + r, 1.0, lr_h), border_color, 0.0);
                    // 右边框：高度内缩 r
                    ctx.fill_rect(Rect::new(x + w - 1.0, y + r, 1.0, lr_h), border_color, 0.0);
                }
            } else {
                // 上边框
                ctx.fill_rect(Rect::new(x, y, w, 1.0), border_color, 0.0);
                // 下边框
                ctx.fill_rect(Rect::new(x, y + h - 1.0, w, 1.0), border_color, 0.0);
                // 左边框
                ctx.fill_rect(Rect::new(x, y + 1.0, 1.0, h - 2.0), border_color, 0.0);
                // 右边框
                ctx.fill_rect(
                    Rect::new(x + w - 1.0, y + 1.0, 1.0, h - 2.0),
                    border_color,
                    0.0,
                );
            }
        }

        let font_size = (24.0_f64).min(bounds.size.height) * 0.44;
        ctx.draw_text(&state.label, bounds, text_color, font_size as f32);
    }

    fn accessibility(&self, state: &Self::State, _ctx: &AccessContext) -> AccessibilityNode {
        AccessibilityNode::none().label(state.label.as_str())
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::context::PaintOp;
    use rgui_core::traits::{AppMessage, PersistState};

    // ── 辅助函数 ──────────────────────────────────────────────

    /// 创建标准测试用的 bounds: (0, 0, 100, 30)
    fn test_bounds() -> Rect {
        Rect::new(0.0, 0.0, 100.0, 30.0)
    }

    /// 创建 ViewContext 用于 view() 测试
    fn test_view_ctx() -> ViewContext {
        ViewContext::new(Size::new(800.0, 600.0))
    }

    /// 创建 MeasureContext 用于 measure() 测试
    fn test_measure_ctx() -> MeasureContext {
        MeasureContext::default()
    }

    /// 创建 AccessContext 用于 accessibility() 测试
    fn test_access_ctx() -> AccessContext {
        AccessContext::new(Rect::new(0.0, 0.0, 800.0, 600.0))
    }

    /// 创建 UpdateContext 用于 update() 测试
    fn test_update_ctx() -> UpdateContext {
        UpdateContext::new()
    }

    // ═══════════════════════════════════════════════════════════
    // State 测试（2 tests）
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn state_default_values() {
        let state = WaBadgeState::default();
        assert_eq!(state.label, "");
        assert_eq!(state.variant, "brand");
        assert_eq!(state.appearance, "accent");
        assert!(!state.pill);
        assert_eq!(state.attention, "none");
    }

    #[test]
    fn state_field_assignment_and_persist() {
        let mut state = WaBadgeState::default();
        state.label = String::from("42");
        state.variant = String::from("success");
        state.appearance = String::from("filled");
        state.pill = true;
        state.attention = String::from("pulse");

        assert_eq!(state.label, "42");
        assert_eq!(state.variant, "success");
        assert_eq!(state.appearance, "filled");
        assert!(state.pill);
        assert_eq!(state.attention, "pulse");

        // PersistState trait 方法可用
        assert_eq!(WaBadgeState::schema_name(), "WaBadgeState");
        assert_eq!(WaBadgeState::schema_version(), 1);

        // as_any / as_any_mut 类型擦除
        let any = state.as_any();
        assert!(any.downcast_ref::<WaBadgeState>().is_some());

        let any_mut = state.as_any_mut();
        assert!(any_mut.downcast_ref::<WaBadgeState>().is_some());
    }

    // ═══════════════════════════════════════════════════════════
    // Message 测试（1 test）
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn message_noop_name() {
        let msg = WaBadgeMessage::NoOp;
        // AppMessage derive 将 NoOp → "no_op"
        assert_eq!(msg.message_name(), "no_op");
    }

    // ═══════════════════════════════════════════════════════════
    // View 测试（2 tests）
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn view_normal_label() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("New"),
            ..WaBadgeState::default()
        };
        let view = wa.view(&state, &test_view_ctx());

        assert_eq!(view.widget_type, "WaBadge");
        assert_eq!(view.children.len(), 0);

        // 验证所有 5 个 prop 存在
        use rgui_core::view::PropValue;
        assert_eq!(view.props.get("label"), Some(&PropValue::Str("New".into())));
        assert_eq!(
            view.props.get("variant"),
            Some(&PropValue::Str("brand".into()))
        );
        assert_eq!(
            view.props.get("appearance"),
            Some(&PropValue::Str("accent".into()))
        );
        assert_eq!(view.props.get("pill"), Some(&PropValue::Bool(false)));
        assert_eq!(
            view.props.get("attention"),
            Some(&PropValue::Str("none".into()))
        );
    }

    #[test]
    fn view_empty_label() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::new(),
            ..WaBadgeState::default()
        };
        let view = wa.view(&state, &test_view_ctx());

        assert_eq!(view.widget_type, "WaBadge");
        // 空 label 时 view 仍正常生成（empty string prop）
        use rgui_core::view::PropValue;
        assert_eq!(view.props.get("label"), Some(&PropValue::Str("".into())));
    }

    // ═══════════════════════════════════════════════════════════
    // Paint 测试（6 tests）
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn paint_accent_appearance() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("A"),
            appearance: String::from("accent"),
            ..WaBadgeState::default()
        };
        let bounds = test_bounds();
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        // accent: 1 fill_rect (bg) + 1 draw_text = 2 ops
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], PaintOp::FillRect { .. }));
        assert!(matches!(ops[1], PaintOp::DrawText { .. }));

        // 验证 bg 颜色和半径
        if let PaintOp::FillRect { color, radius, .. } = &ops[0] {
            assert_eq!(*color, Color::new(0.09, 0.30, 0.90, 1.0));
            assert_eq!(*radius, 4.0_f32); // pill=false → 4.0
        } else {
            panic!("expected FillRect");
        }

        // 验证文本颜色
        if let PaintOp::DrawText { color, .. } = &ops[1] {
            assert_eq!(*color, Color::WHITE);
        } else {
            panic!("expected DrawText");
        }
    }

    #[test]
    fn paint_filled_appearance() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("B"),
            appearance: String::from("filled"),
            ..WaBadgeState::default()
        };
        let bounds = test_bounds();
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        // filled: 1 fill_rect (bg) + 1 draw_text = 2 ops, no border
        assert_eq!(ops.len(), 2);

        if let PaintOp::FillRect { color, .. } = &ops[0] {
            assert_eq!(*color, Color::new(0.85, 0.91, 1.0, 1.0));
        } else {
            panic!("expected FillRect");
        }

        if let PaintOp::DrawText { color, .. } = &ops[1] {
            assert_eq!(*color, Color::new(0.05, 0.10, 0.40, 1.0));
        } else {
            panic!("expected DrawText");
        }
    }

    #[test]
    fn paint_outlined_appearance() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("C"),
            appearance: String::from("outlined"),
            ..WaBadgeState::default()
        };
        let bounds = test_bounds();
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        // outlined: no bg, 4 border rects + 1 draw_text = 5 ops
        assert_eq!(ops.len(), 5);

        // 前 4 个操作应为 FillRect（边框），第 5 个是 DrawText
        for i in 0..4 {
            assert!(matches!(ops[i], PaintOp::FillRect { .. }));
        }
        assert!(matches!(ops[4], PaintOp::DrawText { .. }));

        // 边框颜色验证（top border）
        if let PaintOp::FillRect { color, .. } = &ops[0] {
            assert_eq!(*color, Color::new(0.09, 0.30, 0.90, 1.0));
        }

        // 文本颜色验证
        if let PaintOp::DrawText { color, .. } = &ops[4] {
            assert_eq!(*color, Color::new(0.09, 0.30, 0.90, 1.0));
        }
    }

    #[test]
    fn paint_filled_outlined_appearance() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("D"),
            appearance: String::from("filled-outlined"),
            ..WaBadgeState::default()
        };
        let bounds = test_bounds();
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        // filled-outlined: 1 bg + 4 border + 1 text = 6 ops
        assert_eq!(ops.len(), 6);

        // bg 颜色
        if let PaintOp::FillRect { color, .. } = &ops[0] {
            assert_eq!(*color, Color::new(0.85, 0.91, 1.0, 1.0));
        }

        // 边框（ops[1..5]）
        if let PaintOp::FillRect { color, .. } = &ops[1] {
            assert_eq!(*color, Color::new(0.60, 0.75, 0.95, 1.0));
        }

        // 文本
        if let PaintOp::DrawText { color, .. } = &ops[5] {
            assert_eq!(*color, Color::new(0.05, 0.10, 0.40, 1.0));
        }
    }

    #[test]
    fn paint_pill_true_radius() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("P"),
            appearance: String::from("accent"),
            pill: true,
            ..WaBadgeState::default()
        };
        let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        assert_eq!(ops.len(), 2);

        // pill=true → radius = height/2 = 15.0
        if let PaintOp::FillRect { radius, .. } = &ops[0] {
            assert_eq!(*radius, 15.0_f32);
        } else {
            panic!("expected FillRect for bg");
        }
    }

    #[test]
    fn paint_unknown_appearance_fallback_to_accent() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("X"),
            appearance: String::from("ghost"),
            ..WaBadgeState::default()
        };
        let bounds = test_bounds();
        let mut ctx = PaintContext::new(bounds);

        // 不应 panic
        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        // 降级到 accent: 1 bg + 1 text = 2 ops
        assert_eq!(ops.len(), 2);

        // 背景色应与 accent 一致
        if let PaintOp::FillRect { color, .. } = &ops[0] {
            assert_eq!(*color, Color::new(0.09, 0.30, 0.90, 1.0));
        }
        if let PaintOp::DrawText { color, .. } = &ops[1] {
            assert_eq!(*color, Color::WHITE);
        }
    }

    #[test]
    fn paint_pill_outlined_border_inset() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("Pill"),
            appearance: String::from("outlined"),
            pill: true,
            ..WaBadgeState::default()
        };
        // bounds 100×30，pill radius = 15
        let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        // outlined + pill: no bg, 4 border + 1 text = 5 ops
        // pill 时 radius=15, 左边框/右边框高度 = 30-30=0 被跳过,
        // 上边框/下边框宽度 = 100-30=70 被绘制
        assert_eq!(
            ops.len(),
            3,
            "pill outlined: 2 border (top+bottom) + 1 text"
        );

        // ops[0]: 上边框内缩
        if let PaintOp::FillRect { rect, radius, .. } = &ops[0] {
            assert!(
                (rect.origin.x - 15.0).abs() < 0.01,
                "top border x should be inset by radius=15"
            );
            assert!(
                (rect.size.width - 70.0).abs() < 0.01,
                "top border width should be 100-30=70"
            );
            assert_eq!(*radius, 0.0);
        } else {
            panic!("expected FillRect for top border");
        }

        // ops[1]: 下边框内缩
        if let PaintOp::FillRect { rect, radius, .. } = &ops[1] {
            assert!(
                (rect.origin.x - 15.0).abs() < 0.01,
                "bottom border x should be inset by radius=15"
            );
            assert!(
                (rect.size.width - 70.0).abs() < 0.01,
                "bottom border width should be 100-30=70"
            );
            assert_eq!(*radius, 0.0);
        } else {
            panic!("expected FillRect for bottom border");
        }

        // ops[2]: DrawText
        assert!(matches!(ops[2], PaintOp::DrawText { .. }));
    }

    #[test]
    fn paint_text_centered_bounds() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("T"),
            appearance: String::from("accent"),
            ..WaBadgeState::default()
        };
        // bounds 100×30: font_size = min(24,30)*0.44 = 10.56
        // text_height = 10.56 * 1.448 ≈ 15.29088
        // text_bounds.y = 0 + (30 - 15.29088) / 2 ≈ 7.35456
        let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        let ops = ctx.operations();
        assert_eq!(ops.len(), 2);

        // 验证 DrawText bounds 使用了完整 bounds（不再计算 text_bounds 子矩形）
        if let PaintOp::DrawText {
            bounds: text_bounds,
            font_size,
            ..
        } = &ops[1]
        {
            let expected_font_size = (24.0_f64).min(bounds.size.height) * 0.44;

            assert!((*font_size as f64 - expected_font_size).abs() < 0.01);
            // bounds 应为完整的 badge bounds
            assert!((text_bounds.origin.x - bounds.origin.x).abs() < 0.01);
            assert!((text_bounds.origin.y - bounds.origin.y).abs() < 0.01);
            assert!((text_bounds.size.width - bounds.size.width).abs() < 0.01);
            assert!((text_bounds.size.height - bounds.size.height).abs() < 0.01);
        } else {
            panic!("expected DrawText");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Measure 测试（3 tests）
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn measure_normal_label() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("Hello"),
            ..WaBadgeState::default()
        };
        let constraints = BoxConstraints::new(0.0, f64::MAX, 0.0, 50.0);
        let size = wa.measure(&state, constraints, &test_measure_ctx());

        // font_size = min(50.0, 24.0) * 0.44 = 10.56
        // width = 5 * 10.56 * 0.6 + 20.0 = 51.68
        // height = 10.56 * 1.448 + 12.0 = 27.29088
        assert!((size.width - 51.68).abs() < 0.01);
        assert!((size.height - 27.29088).abs() < 0.01);
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
    }

    #[test]
    fn measure_zero_constraints_returns_zero() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("Hello"),
            ..WaBadgeState::default()
        };
        let constraints = BoxConstraints::new(0.0, 0.0, 0.0, 0.0);
        let size = wa.measure(&state, constraints, &test_measure_ctx());

        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn measure_very_long_text_no_panic() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: "A".repeat(10_000),
            ..WaBadgeState::default()
        };
        let constraints = BoxConstraints::new(0.0, f64::MAX, 0.0, f64::MAX);
        // 不应 panic
        let size = wa.measure(&state, constraints, &test_measure_ctx());

        // 超长文本返回合理的非零尺寸
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
        assert!(!size.is_empty());
    }

    // ═══════════════════════════════════════════════════════════
    // Accessibility 测试（3 tests）
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn accessibility_normal_label() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("42"),
            ..WaBadgeState::default()
        };
        let node = wa.accessibility(&state, &test_access_ctx());

        assert_eq!(node.role, rgui_core::a11y::AccessibilityRole::None);
        assert_eq!(node.label.as_deref(), Some("42"));
    }

    #[test]
    fn accessibility_empty_label() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::new(),
            ..WaBadgeState::default()
        };
        let node = wa.accessibility(&state, &test_access_ctx());

        assert_eq!(node.role, rgui_core::a11y::AccessibilityRole::None);
        assert_eq!(node.label.as_deref(), Some(""));
    }

    #[test]
    fn accessibility_unknown_appearance_no_effect() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("Tags"),
            appearance: String::from("custom-style"),
            ..WaBadgeState::default()
        };
        let node = wa.accessibility(&state, &test_access_ctx());

        // accessibility 不依赖 appearance，仅传 label
        assert_eq!(node.role, rgui_core::a11y::AccessibilityRole::None);
        assert_eq!(node.label.as_deref(), Some("Tags"));
    }

    // ═══════════════════════════════════════════════════════════
    // 边界条件测试
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn paint_empty_label_early_return() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::new(),
            ..WaBadgeState::default()
        };
        let bounds = test_bounds();
        let mut ctx = PaintContext::new(bounds);

        wa.paint(&state, bounds, &mut ctx);

        // 空 label → 零绘制操作
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn paint_zero_bounds_early_return() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::from("X"),
            ..WaBadgeState::default()
        };
        let mut ctx = PaintContext::new(Rect::ZERO);

        wa.paint(&state, Rect::ZERO, &mut ctx);

        // 零尺寸 bounds → 零绘制操作
        assert_eq!(ctx.op_count(), 0);
    }

    #[test]
    fn measure_empty_label_returns_zero() {
        let wa = WaBadge;
        let state = WaBadgeState {
            label: String::new(),
            ..WaBadgeState::default()
        };
        let constraints = BoxConstraints::UNCONSTRAINED;
        let size = wa.measure(&state, constraints, &test_measure_ctx());

        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn update_noop_does_not_panic() {
        let wa = WaBadge;
        let mut state = WaBadgeState {
            label: String::from("test"),
            ..WaBadgeState::default()
        };
        let mut ctx = test_update_ctx();

        // NoOp 消息不应修改 state 或 panic
        wa.update(WaBadgeMessage::NoOp, &mut state, &mut ctx);
        assert_eq!(state.label, "test");
    }
}
