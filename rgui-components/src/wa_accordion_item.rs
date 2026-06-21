/// Translated from Web Awesome accordion-item
/// Original license: MIT
/// Copyright (c) Font Awesome
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

/// Web Awesome wa-accordion-item 组件状态。
///
/// AccordionItem 是可展开/折叠的面板项，包含标题栏和可折叠内容区域。
///
/// 简化项（Phase 0）：
/// - 展开/折叠动画 → 即时切换
/// - 图标旋转动画 → Unicode 字符静态切换
/// - RTL 方向 → 硬编码 LTR
/// - headingLevel 动态包装 → 始终渲染为纯按钮（无 h1-h6 包装）
/// - 命名 slot（label/icon）→ 使用 state.label 文本
/// - isTabbable / 键盘导航 → rgui 无焦点管理
/// - isAnimating → 跳过（无动画系统）
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaAccordionItemState {
    /// 标题文本（对应 label prop 和 label slot 回退）
    pub label: String,
    /// 是否展开
    pub expanded: bool,
    /// 是否禁用
    pub disabled: bool,
    /// 展开/折叠图标位置：start | end
    pub icon_placement: String,
    /// 视觉外观：filled | outlined | filled-outlined | plain
    pub appearance: String,
    /// 标题级别（1-6 或 none），由父 Accordion 设置
    pub heading_level: String,
    /// 可折叠内容文本（展开时渲染）
    pub content: String,
}

impl WaAccordionItemState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            label: String::new(),
            expanded: false,
            disabled: false,
            icon_placement: "end".into(),
            appearance: "outlined".into(),
            heading_level: "3".into(),
            content: String::new(),
        }
    }
}

// ============================================================================
// Message
// ============================================================================

/// AccordionItem 事件：
/// - `Trigger` — 触发器被点击（发送给父 Accordion 协调模式逻辑）
/// - `Expanded` — 展开动画完成
/// - `Collapsed` — 折叠动画完成
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaAccordionItemMessage {
    Trigger,
    Expanded,
    Collapsed,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

/// 标题栏高度（与 WA 源 --wa-space-m padding 对齐，约 44px）
const HEADER_HEIGHT: f64 = 44.0;
/// 字体大小比例
const FONT_RATIO: f64 = 0.44;

pub struct WaAccordionItem;

impl WidgetSpec for WaAccordionItem {
    type State = WaAccordionItemState;
    type Message = WaAccordionItemMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaAccordionItem"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaAccordionItem")
            .prop(
                "label",
                PropValue::str(if state.label.is_empty() {
                    ""
                } else {
                    state.label.as_str()
                }),
            )
            .prop("expanded", PropValue::Bool(state.expanded))
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop(
                "icon-placement",
                PropValue::str(state.icon_placement.as_str()),
            )
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop(
                "heading-level",
                PropValue::str(state.heading_level.as_str()),
            )
            .prop("content", PropValue::str(state.content.as_str()))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaAccordionItemMessage::Trigger => {
                if !state.disabled {
                    state.expanded = !state.expanded;
                }
            },
            WaAccordionItemMessage::Expanded => {
                state.expanded = true;
            },
            WaAccordionItemMessage::Collapsed => {
                state.expanded = false;
            },
        }
    }

    /// AccordionItem 是叶子组件，尺寸由 Taffy 布局决定。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        Size::ZERO
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let w: f64 = bounds.size.width;
        let h: f64 = bounds.size.height;

        if w < 8.0 || h < HEADER_HEIGHT {
            return;
        }

        let font_size: f32 = (HEADER_HEIGHT * FONT_RATIO) as f32;

        // —— 标题栏背景 ——
        let header_bg = if state.disabled {
            Color::new(0.95, 0.95, 0.95, 1.0)
        } else {
            match state.appearance.as_str() {
                "filled" => Color::new(0.92, 0.92, 0.92, 1.0),
                "filled-outlined" => Color::new(0.92, 0.92, 0.92, 1.0),
                "outlined" => Color::WHITE,
                "plain" => Color::TRANSPARENT,
                _ => Color::WHITE,
            }
        };

        let header_rect = Rect::new(bounds.origin.x, bounds.origin.y, w, HEADER_HEIGHT);
        ctx.fill_rect(header_rect, header_bg, 0.0);

        // —— 标题栏底部分隔线（outlined / filled-outlined 外观） ——
        if state.appearance != "plain" {
            let divider_color = Color::new(0.85, 0.85, 0.85, 1.0);
            let divider_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y + HEADER_HEIGHT - 1.0,
                w,
                1.0,
            );
            ctx.fill_rect(divider_rect, divider_color, 0.0);
        }

        // —— 标签文本 ——
        let text_color = if state.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            Color::new(0.1, 0.1, 0.1, 1.0) // WA --wa-color-text-normal
        };

        let text_label = if state.label.is_empty() {
            ""
        } else {
            state.label.as_str()
        };

        // icon 在 end: 文本在左，icon 在右
        // icon 在 start: icon 在左，文本在右
        let text_x = if state.icon_placement == "start" {
            bounds.origin.x + 36.0 // 留 icon 空间
        } else {
            bounds.origin.x + 16.0 // 标准左边距
        };
        let text_w = w - 52.0; // 减去 icon 宽度(24) + 间距
        let text_rect = Rect::new(text_x, bounds.origin.y, text_w, HEADER_HEIGHT);
        ctx.draw_text(text_label, text_rect, text_color, font_size);

        // —— 展开/折叠图标（Unicode chevron） ——
        let chevron = if state.expanded {
            "\u{25BC}"
        } else {
            "\u{25B6}"
        }; // ▼ or ▶
        let chevron_color = if state.disabled {
            Color::new(0.7, 0.7, 0.7, 1.0)
        } else {
            Color::new(0.5, 0.5, 0.5, 1.0)
        };
        let icon_x = if state.icon_placement == "start" {
            bounds.origin.x + 12.0
        } else {
            bounds.origin.x + w - 32.0
        };
        let icon_rect = Rect::new(icon_x, bounds.origin.y, 24.0, HEADER_HEIGHT);
        ctx.draw_text(chevron, icon_rect, chevron_color, font_size);

        // —— 可折叠内容面板 ——
        if state.expanded {
            let panel_y: f64 = bounds.origin.y + HEADER_HEIGHT;
            let panel_h: f64 = h - HEADER_HEIGHT;
            if panel_h > 0.0 {
                let panel_bg = match state.appearance.as_str() {
                    "filled" => Color::new(0.88, 0.88, 0.88, 1.0),
                    "filled-outlined" => Color::new(0.96, 0.96, 0.96, 1.0),
                    "plain" => Color::TRANSPARENT,
                    _ => Color::new(0.98, 0.98, 0.98, 1.0), // outlined: very light
                };
                let panel_rect = Rect::new(bounds.origin.x, panel_y, w, panel_h);
                ctx.fill_rect(panel_rect, panel_bg, 0.0);

                // 渲染内容文本
                if !state.content.is_empty() {
                    let content_font_size: f32 = (HEADER_HEIGHT * 0.36) as f32;
                    let content_rect = Rect::new(
                        bounds.origin.x + 16.0,
                        panel_y + 8.0,
                        w - 32.0,
                        panel_h - 16.0,
                    );
                    ctx.draw_text(
                        state.content.as_str(),
                        content_rect,
                        Color::new(0.2, 0.2, 0.2, 1.0),
                        content_font_size,
                    );
                }
            }
        }
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = if state.expanded {
            AccessibilityRole::Button // expanded toggle button
        } else {
            AccessibilityRole::Button
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO).label(
            if state.label.is_empty() {
                "accordion item"
            } else {
                state.label.as_str()
            },
        )
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    fn make_ctx() -> ViewContext {
        ViewContext::new(Size::new(800.0, 600.0))
    }

    #[test]
    fn name() {
        assert_eq!(WaAccordionItem.name(), "rgui_components::WaAccordionItem");
    }

    #[test]
    fn default_state() {
        let s = WaAccordionItemState::new();
        assert!(!s.expanded);
        assert!(!s.disabled);
        assert_eq!(s.icon_placement, "end");
        assert_eq!(s.appearance, "outlined");
        assert_eq!(s.heading_level, "3");
    }

    #[test]
    fn state_with_label() {
        let s = WaAccordionItemState {
            label: "Section 1".into(),
            ..WaAccordionItemState::new()
        };
        assert_eq!(s.label, "Section 1");
    }

    #[test]
    fn state_expanded() {
        let s = WaAccordionItemState {
            expanded: true,
            ..WaAccordionItemState::new()
        };
        assert!(s.expanded);
    }

    #[test]
    fn state_disabled() {
        let s = WaAccordionItemState {
            disabled: true,
            ..WaAccordionItemState::new()
        };
        assert!(s.disabled);
    }

    #[test]
    fn update_trigger_toggles() {
        let mut s = WaAccordionItemState::new();
        assert!(!s.expanded);
        WaAccordionItem.update(
            WaAccordionItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.expanded);
        WaAccordionItem.update(
            WaAccordionItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.expanded);
    }

    #[test]
    fn update_trigger_ignored_when_disabled() {
        let mut s = WaAccordionItemState {
            disabled: true,
            ..WaAccordionItemState::new()
        };
        WaAccordionItem.update(
            WaAccordionItemMessage::Trigger,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.expanded, "disabled item should not toggle");
    }

    #[test]
    fn update_expanded_sets_true() {
        let mut s = WaAccordionItemState::new();
        WaAccordionItem.update(
            WaAccordionItemMessage::Expanded,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.expanded);
    }

    #[test]
    fn update_collapsed_sets_false() {
        let mut s = WaAccordionItemState {
            expanded: true,
            ..WaAccordionItemState::new()
        };
        WaAccordionItem.update(
            WaAccordionItemMessage::Collapsed,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(!s.expanded);
    }

    #[test]
    fn measure_returns_zero() {
        let s = WaAccordionItemState::new();
        let size = WaAccordionItem.measure(
            &s,
            BoxConstraints::new(0.0, 800.0, 0.0, 600.0),
            &MeasureContext::default(),
        );
        assert_eq!(size, Size::ZERO);
    }

    #[test]
    fn paint_collapsed_produces_ops() {
        let s = WaAccordionItemState {
            label: "Test".into(),
            ..WaAccordionItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 44.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "collapsed item should have header ops");
    }

    #[test]
    fn paint_expanded_produces_more_ops() {
        let s = WaAccordionItemState {
            label: "Test".into(),
            expanded: true,
            ..WaAccordionItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        // expanded: header + panel → more ops than collapsed
        let expanded_ops = ctx.op_count();

        let s2 = WaAccordionItemState {
            label: "Test".into(),
            ..WaAccordionItemState::new()
        };
        let bounds2 = Rect::new(0.0, 0.0, 400.0, 200.0);
        let mut ctx2 = PaintContext::new(bounds2);
        WaAccordionItem.paint(&s2, bounds2, &mut ctx2);
        assert!(
            expanded_ops > ctx2.op_count(),
            "expanded should produce more ops"
        );
    }

    #[test]
    fn paint_disabled_style() {
        let s = WaAccordionItemState {
            label: "Test".into(),
            disabled: true,
            ..WaAccordionItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 44.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "disabled item should still render");
    }

    #[test]
    fn paint_too_small_returns_early() {
        let s = WaAccordionItemState::new();
        let bounds = Rect::new(0.0, 0.0, 4.0, 4.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "too small bounds should produce nothing");
    }

    #[test]
    fn paint_appearance_filled() {
        let s = WaAccordionItemState {
            label: "Filled".into(),
            appearance: "filled".into(),
            ..WaAccordionItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 44.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0);
    }

    #[test]
    fn paint_appearance_plain() {
        let s = WaAccordionItemState {
            label: "Plain".into(),
            appearance: "plain".into(),
            ..WaAccordionItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 44.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0);
    }

    #[test]
    fn paint_icon_placement_start() {
        let s = WaAccordionItemState {
            label: "Icon Start".into(),
            icon_placement: "start".into(),
            ..WaAccordionItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 400.0, 44.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0);
    }

    #[test]
    fn paint_empty_label() {
        let s = WaAccordionItemState::new();
        let bounds = Rect::new(0.0, 0.0, 400.0, 44.0);
        let mut ctx = PaintContext::new(bounds);
        WaAccordionItem.paint(&s, bounds, &mut ctx);
        assert!(ctx.op_count() > 0, "empty label should still render header");
    }

    #[test]
    fn view_contains_props() {
        let s = WaAccordionItemState {
            label: "Section".into(),
            expanded: true,
            ..WaAccordionItemState::new()
        };
        let v = WaAccordionItem.view(&s, &make_ctx());
        assert!(v.props.contains_key("label"));
        assert!(v.props.contains_key("expanded"));
        assert!(v.props.contains_key("disabled"));
        assert!(v.props.contains_key("icon-placement"));
        assert!(v.props.contains_key("appearance"));
        assert!(v.props.contains_key("heading-level"));
    }

    #[test]
    fn derive_msg() {
        assert_eq!(WaAccordionItemMessage::Trigger.message_name(), "trigger");
    }

    #[test]
    fn derive_state() {
        assert_eq!(WaAccordionItemState::schema_name(), "WaAccordionItemState");
    }

    #[test]
    fn persist_state_as_any() {
        let state = WaAccordionItemState::new();
        let any = state.as_any();
        assert_eq!(any.type_id(), TypeId::of::<WaAccordionItemState>());
    }

    #[test]
    fn accessibility_default_label() {
        let s = WaAccordionItemState::new();
        let node = WaAccordionItem.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("accordion item"));
    }

    #[test]
    fn accessibility_with_label() {
        let s = WaAccordionItemState {
            label: "Section 1".into(),
            ..WaAccordionItemState::new()
        };
        let node = WaAccordionItem.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Section 1"));
    }
}
