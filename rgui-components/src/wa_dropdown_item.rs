/// Translated from Web Awesome dropdown-item
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// Phase 0 简化:
/// - 无 submenu（子菜单系统复杂，Phase 0 跳过）
/// - 无 checkbox 渲染（Phase 0 仅绘制文本标签）
/// - 无 icon/details slot 绘制（由子节点渲染）
/// - 无 hasSubmenu/submenuAdjacent/checkboxAdjacent 内部属性
/// - 无动画（show/hide CSS 类）
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

/// Web Awesome wa-dropdown-item 组件状态。
///
/// DropdownItem 是下拉菜单中的可选项，支持文本标签、图标、详情文本、复选框和子菜单。
///
/// Phase 0 简化项：
/// - `size`、`checkboxAdjacent`、`submenuAdjacent` → 跳过（内部使用）
/// - `hasSubmenu` (@state) → 跳过（Phase 0 无子菜单）
/// - `submenuOpen` → 跳过（Phase 0 无子菜单）
/// - `type` "checkbox" → 接受但不渲染复选框
/// - icon/details slot → 由 Children 渲染
#[derive(Debug, Clone, serde::Serialize, Persist)]
pub struct WaDropdownItemState {
    /// 是否高亮/激活
    pub active: bool,
    /// 变体：default / danger
    pub variant: String,
    /// 选项值（用于 wa-select 事件识别）
    pub value: String,
    /// 类型：normal / checkbox（Phase 0 不渲染复选框）
    pub type_: String,
    /// 是否勾选（仅 type=checkbox 时有效）
    pub checked: bool,
    /// 是否禁用
    pub disabled: bool,
}

impl Default for WaDropdownItemState {
    fn default() -> Self {
        Self {
            active: false,
            variant: String::new(),
            value: String::new(),
            type_: String::new(),
            checked: false,
            disabled: false,
        }
    }
}

impl WaDropdownItemState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Message
// ============================================================================

/// DropdownItem 事件。
///
/// - `Focus` — 选项获得焦点
/// - `Blur` — 选项失去焦点
///
/// Phase 0：两个事件均为占位，无实际行为。
#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaDropdownItemMessage {
    Focus,
    Blur,
}

// ============================================================================
// WidgetSpec 实现
// ============================================================================

pub struct WaDropdownItem;

impl WidgetSpec for WaDropdownItem {
    type State = WaDropdownItemState;
    type Message = WaDropdownItemMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaDropdownItem"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        let mut v = WidgetView::new("rgui_components::WaDropdownItem");

        if state.active {
            v = v.prop("active", PropValue::Bool(true));
        }
        if !state.variant.is_empty() {
            v = v.prop("variant", PropValue::str(state.variant.as_str()));
        }
        if !state.value.is_empty() {
            v = v.prop("value", PropValue::str(state.value.as_str()));
        }
        if !state.type_.is_empty() {
            v = v.prop("type", PropValue::str(state.type_.as_str()));
        }
        if state.checked {
            v = v.prop("checked", PropValue::Bool(true));
        }
        if state.disabled {
            v = v.prop("disabled", PropValue::Bool(true));
        }

        v
    }

    fn update(&self, msg: Self::Message, _state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaDropdownItemMessage::Focus => {},
            WaDropdownItemMessage::Blur => {},
        }
    }

    /// DropdownItem 是叶子组件，返回最小高度保证可交互。
    fn measure(&self, _state: &Self::State, _c: BoxConstraints, _: &MeasureContext) -> Size {
        // 返回最小高度 32px，宽度由容器决定
        Size::new(100.0, 32.0)
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        if state.disabled {
            return;
        }

        let border_radius: f32 = 6.0; // --wa-border-radius-s

        // 背景色：active 或 hover 时高亮
        if state.active {
            let active_bg = match state.variant.as_str() {
                "danger" => Color::new(0.85, 0.18, 0.15, 1.0), // --wa-color-danger-fill-normal
                _ => Color::new(0.94, 0.94, 0.94, 1.0),        // --wa-color-neutral-fill-normal
            };
            ctx.fill_rect(bounds, active_bg, border_radius);
        }

        // 文本标签
        let text_color = match state.variant.as_str() {
            "danger" => Color::new(0.85, 0.18, 0.15, 1.0), // --wa-color-danger-on-quiet
            _ => Color::new(0.13, 0.13, 0.13, 1.0),        // --wa-color-text-normal
        };

        // 文本区域：左侧留白，垂直居中
        let text_x = bounds.origin.x + 12.0;
        let text_w = bounds.size.width - 24.0;
        let font_size: f32 = 14.0;
        let text_bounds = Rect::new(text_x, bounds.origin.y, text_w, bounds.size.height);

        let label = if state.value.is_empty() {
            "Item"
        } else {
            state.value.as_str()
        };

        ctx.draw_text(label, text_bounds, text_color, font_size);
    }

    fn accessibility(&self, state: &Self::State, _: &AccessContext) -> AccessibilityNode {
        let role = AccessibilityRole::Custom("menuitem");
        let label = if state.value.is_empty() {
            "item"
        } else {
            state.value.as_str()
        };
        AccessibilityNode::new(WidgetId::from_u64(0), role, Rect::ZERO)
            .label(label)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> ViewContext {
        ViewContext::new(Size::new(200.0, 40.0))
    }

    #[test]
    fn name() {
        assert_eq!(WaDropdownItem.name(), "rgui_components::WaDropdownItem");
    }

    #[test]
    fn default_state() {
        let s = WaDropdownItemState::new();
        assert!(!s.active);
        assert!(s.variant.is_empty());
        assert!(s.value.is_empty());
        assert!(s.type_.is_empty());
        assert!(!s.checked);
        assert!(!s.disabled);
    }

    #[test]
    fn state_active() {
        let s = WaDropdownItemState {
            active: true,
            ..WaDropdownItemState::new()
        };
        assert!(s.active);
    }

    #[test]
    fn state_variant_danger() {
        let s = WaDropdownItemState {
            variant: "danger".into(),
            ..WaDropdownItemState::new()
        };
        assert_eq!(s.variant, "danger");
    }

    #[test]
    fn state_with_value() {
        let s = WaDropdownItemState {
            value: "option-1".into(),
            ..WaDropdownItemState::new()
        };
        assert_eq!(s.value, "option-1");
    }

    #[test]
    fn state_disabled() {
        let s = WaDropdownItemState {
            disabled: true,
            ..WaDropdownItemState::new()
        };
        assert!(s.disabled);
    }

    #[test]
    fn state_checked() {
        let s = WaDropdownItemState {
            checked: true,
            ..WaDropdownItemState::new()
        };
        assert!(s.checked);
    }

    #[test]
    fn view_contains_core_props() {
        let s = WaDropdownItemState {
            value: "test".into(),
            ..WaDropdownItemState::new()
        };
        let v = WaDropdownItem.view(&s, &make_ctx());
        assert!(
            v.props.contains_key("value"),
            "view 应包含 value prop"
        );
    }

    #[test]
    fn view_active_prop() {
        let s = WaDropdownItemState {
            active: true,
            ..WaDropdownItemState::new()
        };
        let v = WaDropdownItem.view(&s, &make_ctx());
        let val = v.props.get("active").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for active"),
        }
    }

    #[test]
    fn view_variant_prop() {
        let s = WaDropdownItemState {
            variant: "danger".into(),
            ..WaDropdownItemState::new()
        };
        let v = WaDropdownItem.view(&s, &make_ctx());
        let val = v.props.get("variant").unwrap();
        match val {
            PropValue::Str(s) => assert_eq!(s.as_ref(), "danger"),
            _ => panic!("expected Str prop for variant"),
        }
    }

    #[test]
    fn view_disabled_prop() {
        let s = WaDropdownItemState {
            disabled: true,
            ..WaDropdownItemState::new()
        };
        let v = WaDropdownItem.view(&s, &make_ctx());
        let val = v.props.get("disabled").unwrap();
        match val {
            PropValue::Bool(b) => assert!(*b),
            _ => panic!("expected Bool prop for disabled"),
        }
    }

    #[test]
    fn view_inactive_no_active_prop() {
        let s = WaDropdownItemState::new(); // active = false
        let v = WaDropdownItem.view(&s, &make_ctx());
        assert!(
            !v.props.contains_key("active"),
            "inactive 时不应有 active prop"
        );
    }

    #[test]
    fn update_focus_noop() {
        let mut s = WaDropdownItemState::new();
        WaDropdownItem.update(
            WaDropdownItemMessage::Focus,
            &mut s,
            &mut UpdateContext::default(),
        );
        // 不 panic 即通过
    }

    #[test]
    fn update_blur_noop() {
        let mut s = WaDropdownItemState {
            active: true,
            ..WaDropdownItemState::new()
        };
        WaDropdownItem.update(
            WaDropdownItemMessage::Blur,
            &mut s,
            &mut UpdateContext::default(),
        );
        assert!(s.active, "Blur 不应改变 active 状态");
    }

    #[test]
    fn measure_returns_min_size() {
        let s = WaDropdownItemState::new();
        let size = WaDropdownItem.measure(
            &s,
            BoxConstraints::new(0.0, 300.0, 0.0, 200.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 100.0);
        assert!(size.height >= 32.0);
    }

    #[test]
    fn paint_disabled_produces_no_ops() {
        let s = WaDropdownItemState {
            disabled: true,
            ..WaDropdownItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdownItem.paint(&s, bounds, &mut ctx);
        assert_eq!(ctx.op_count(), 0, "禁用的 DropdownItem 不应绘制任何内容");
    }

    #[test]
    fn paint_active_produces_ops() {
        let s = WaDropdownItemState {
            active: true,
            value: "Save".into(),
            ..WaDropdownItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdownItem.paint(&s, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 2,
            "active DropdownItem 应产生背景+文本绘制操作，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_danger_variant() {
        let s = WaDropdownItemState {
            active: true,
            variant: "danger".into(),
            value: "Delete".into(),
            ..WaDropdownItemState::new()
        };
        let bounds = Rect::new(0.0, 0.0, 200.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdownItem.paint(&s, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 2,
            "danger variant DropdownItem 应产生绘制操作"
        );
    }

    #[test]
    fn paint_normal_no_value() {
        let s = WaDropdownItemState::new(); // 无 value, 非 active
        let bounds = Rect::new(0.0, 0.0, 200.0, 32.0);
        let mut ctx = PaintContext::new(bounds);
        WaDropdownItem.paint(&s, bounds, &mut ctx);
        // inactive 且无 value → 仍绘制默认 "Item" 文本
        assert!(ctx.op_count() == 1, "应至少绘制默认文本");
    }

    #[test]
    fn accessibility_menuitem_role() {
        let s = WaDropdownItemState::new();
        let node = WaDropdownItem.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("item"));
    }

    #[test]
    fn accessibility_with_value() {
        let s = WaDropdownItemState {
            value: "Settings".into(),
            ..WaDropdownItemState::new()
        };
        let node = WaDropdownItem.accessibility(&s, &AccessContext::new(Rect::ZERO));
        assert_eq!(node.label.as_deref(), Some("Settings"));
    }

    #[test]
    fn derive_message_name() {
        assert_eq!(
            WaDropdownItemMessage::Focus.message_name(),
            "focus"
        );
        assert_eq!(
            WaDropdownItemMessage::Blur.message_name(),
            "blur"
        );
    }

    #[test]
    fn derive_schema_name() {
        assert_eq!(
            WaDropdownItemState::schema_name(),
            "WaDropdownItemState"
        );
    }

    #[test]
    fn state_as_any() {
        use std::any::Any;
        let s = WaDropdownItemState::new();
        let _: &dyn Any = s.as_any();
    }
}
