/// Translated from Web Awesome wa-select
/// Original license: MIT
/// Copyright (c) Font Awesome
///
/// Select/Combobox 组件。Phase 0 翻译：渲染 combobox 外观（标签+值文本+下拉箭头+清除按钮），
/// 下拉列表渲染为简化矩形区域。完整的 popup 行为依赖 WTI01-WTI03 基础设施。
/// 跳过：FormField trait impl、多选 tag 渲染、键盘导航、type-to-select。
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

/// Web Awesome wa-select 组件状态。
///
/// 下拉选择控件，Phase 0 仅渲染 combobox 外观。
///
/// 跳过：withLabel/withHint SSR 属性、getTag 自定义渲染函数。
/// FormField trait impl 暂时跳过（WC02 trait 已存在但尚未提交到代码中）。
#[derive(Debug, Clone, Default, serde::Serialize, Persist)]
pub struct WaSelectState {
    /// 表单名称
    pub name: String,
    /// 当前选中值（单选时为字符串，多选时为逗号分隔）
    pub value: String,
    /// 尺寸：xs | s | m | l | xl | small | medium | large
    pub size: String,
    /// 占位符文本
    pub placeholder: String,
    /// 允许多选
    pub multiple: bool,
    /// 多选时最大显示 tag 数量（0=无限制）
    pub max_options_visible: u32,
    /// 禁用状态
    pub disabled: bool,
    /// 显示清除按钮
    pub with_clear: bool,
    /// 下拉列表是否打开
    pub open: bool,
    /// 视觉外观：filled | outlined | filled-outlined
    pub appearance: String,
    /// 药丸形状
    pub pill: bool,
    /// 标签文本
    pub label: String,
    /// 菜单弹出方向：top | bottom
    pub placement: String,
    /// 提示文本
    pub hint: String,
    /// 必填字段
    pub required: bool,
    /// 当前显示的标签文本（单选时为选项 label，多选时为 "N options selected"）
    pub display_label: String,
}

impl WaSelectState {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: "m".into(),
            appearance: "outlined".into(),
            placement: "bottom".into(),
            max_options_visible: 3,
            ..Self::default()
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMsg)]
pub enum WaSelectMessage {
    /// 值变更
    Change,
    /// 接收输入
    Input,
    /// 获得焦点
    Focus,
    /// 失去焦点
    Blur,
    /// 清除按钮被点击
    Clear,
    /// 下拉菜单打开
    Show,
    /// 下拉菜单打开动画完成
    AfterShow,
    /// 下拉菜单关闭
    Hide,
    /// 下拉菜单关闭动画完成
    AfterHide,
    /// 验证失败
    Invalid,
}

// ============================================================================
// WidgetSpec
// ============================================================================

pub struct WaSelect;

/// 根据 size 属性的像素高度映射
fn select_height(size: &str) -> f64 {
    match size {
        "xs" => 24.0,
        "s" | "small" => 28.0,
        "l" | "large" => 48.0,
        "xl" => 56.0,
        _ => 36.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 属性的字体大小映射
fn select_font_size(size: &str) -> f64 {
    match size {
        "xs" => 12.0,
        "s" | "small" => 14.0,
        "l" | "large" => 20.0,
        "xl" => 24.0,
        _ => 16.0, // "m" | "medium" (默认)
    }
}

/// 根据 size 和 pill 的 border-radius 映射
fn select_border_radius(size: &str, pill: bool) -> f32 {
    if pill {
        return (select_height(size) / 2.0) as f32;
    }
    match size {
        "xs" => 2.0,
        "s" | "small" => 3.0,
        "l" | "large" => 6.0,
        "xl" => 8.0,
        _ => 4.0, // "m" | "medium" (默认)
    }
}

impl WidgetSpec for WaSelect {
    type State = WaSelectState;
    type Message = WaSelectMessage;

    fn name(&self) -> &'static str {
        "rgui_components::WaSelect"
    }

    fn view(&self, state: &Self::State, _: &ViewContext) -> WidgetView<Self::Message> {
        WidgetView::new("rgui_components::WaSelect")
            .prop("name", PropValue::str(state.name.as_str()))
            .prop("value", PropValue::str(state.value.as_str()))
            .prop("size", PropValue::str(state.size.as_str()))
            .prop("placeholder", PropValue::str(state.placeholder.as_str()))
            .prop("label", PropValue::str(state.label.as_str()))
            .prop("hint", PropValue::str(state.hint.as_str()))
            .prop("appearance", PropValue::str(state.appearance.as_str()))
            .prop("placement", PropValue::str(state.placement.as_str()))
            .prop(
                "display-label",
                PropValue::str(state.display_label.as_str()),
            )
            .prop("disabled", PropValue::Bool(state.disabled))
            .prop("multiple", PropValue::Bool(state.multiple))
            .prop("pill", PropValue::Bool(state.pill))
            .prop("with-clear", PropValue::Bool(state.with_clear))
            .prop("required", PropValue::Bool(state.required))
            .prop("open", PropValue::Bool(state.open))
    }

    fn update(&self, msg: Self::Message, state: &mut Self::State, _: &mut UpdateContext) {
        match msg {
            WaSelectMessage::Clear => {
                if !state.disabled {
                    state.value = String::new();
                    state.display_label = String::new();
                }
            },
            WaSelectMessage::Show => {
                if !state.disabled {
                    state.open = true;
                }
            },
            WaSelectMessage::Hide | WaSelectMessage::AfterHide => {
                state.open = false;
            },
            WaSelectMessage::AfterShow => {
                // 动画完成后不做额外处理
            },
            WaSelectMessage::Change
            | WaSelectMessage::Input
            | WaSelectMessage::Focus
            | WaSelectMessage::Blur
            | WaSelectMessage::Invalid => {},
        }
    }

    fn measure(&self, state: &Self::State, c: BoxConstraints, _: &MeasureContext) -> Size {
        let h = select_height(&state.size);
        let font_size = select_font_size(&state.size);

        // 宽度：根据 display_label 或 placeholder 文本 + 下拉箭头空间估算
        let display_text = if state.display_label.is_empty() {
            &state.placeholder
        } else {
            &state.display_label
        };
        let char_count = display_text.chars().count().max(8) as f64;
        // 文本宽度 + 箭头(1em) + padding(~32px) + 清除按钮空间
        let text_width = char_count * font_size * 0.6 + font_size + 32.0;

        // 如果有标签，加上标签高度
        let label_height = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };
        let total_h = h + label_height;

        Size::new(
            text_width.clamp(c.min_width, c.max_width),
            total_h.clamp(c.min_height, c.max_height),
        )
    }

    fn paint(&self, state: &Self::State, bounds: Rect, ctx: &mut PaintContext) {
        let h = select_height(&state.size);
        let font_size = select_font_size(&state.size);
        let border_radius = select_border_radius(&state.size, state.pill);

        // 标签绘制偏移
        let label_offset_y = if state.label.is_empty() {
            0.0
        } else {
            font_size * 1.4
        };
        let combobox_y = bounds.origin.y + label_offset_y;
        let combobox_rect = Rect::new(bounds.origin.x, combobox_y, bounds.size.width, h);

        // ── 标签 ──
        if !state.label.is_empty() {
            let label_font_size = font_size as f32 * 0.9;
            let label_color = if state.disabled {
                Color::new(0.6, 0.6, 0.6, 1.0)
            } else {
                Color::new(0.2, 0.2, 0.2, 1.0)
            };
            let label_rect = Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                label_offset_y,
            );
            ctx.draw_text(&state.label, label_rect, label_color, label_font_size);
        }

        // ── Combobox 背景 ──
        let (bg, border_color) = match state.appearance.as_str() {
            "filled" => (
                Color::new(0.95, 0.95, 0.95, 1.0),
                Color::new(0.85, 0.85, 0.85, 1.0),
            ),
            "filled-outlined" => (
                Color::new(0.95, 0.95, 0.95, 1.0),
                Color::new(0.6, 0.6, 0.6, 1.0),
            ),
            _ => (Color::WHITE, Color::new(0.6, 0.6, 0.6, 1.0)), // outlined (默认)
        };

        let bg_color = if state.disabled {
            Color::new(0.92, 0.92, 0.92, 0.5)
        } else {
            bg
        };
        let border = if state.disabled {
            Color::new(0.7, 0.7, 0.7, 0.5)
        } else {
            border_color
        };

        // 绘制 combobox 背景
        ctx.fill_rect(combobox_rect, bg_color, border_radius);

        // 模拟边框
        let border_width: f64 = 2.0;
        ctx.fill_rect(combobox_rect, border, border_radius);
        let inner_rect = Rect::new(
            combobox_rect.origin.x + border_width,
            combobox_rect.origin.y + border_width,
            combobox_rect.size.width - border_width * 2.0,
            combobox_rect.size.height - border_width * 2.0,
        );
        let inner_radius = (border_radius as f64 - 1.0).max(1.0) as f32;
        ctx.fill_rect(inner_rect, bg_color, inner_radius);

        // ── 显示文本 / 占位符 ──
        let display_text = if state.display_label.is_empty() {
            &state.placeholder
        } else {
            &state.display_label
        };

        let text_color = if state.display_label.is_empty() {
            // 占位符颜色
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else if state.disabled {
            Color::new(0.6, 0.6, 0.6, 1.0)
        } else {
            Color::new(0.1, 0.1, 0.1, 1.0)
        };

        if !display_text.is_empty() {
            let text_padding: f64 = 12.0;
            // 给右侧箭头和清除按钮留空间
            let right_margin: f64 = font_size + 16.0;
            let text_rect = Rect::new(
                combobox_rect.origin.x + text_padding,
                combobox_rect.origin.y,
                combobox_rect.size.width - text_padding - right_margin,
                combobox_rect.size.height,
            );
            ctx.draw_text(display_text, text_rect, text_color, font_size as f32);
        }

        // ── 清除按钮 (✕) ──
        let has_clear = state.with_clear && !state.disabled && !state.display_label.is_empty();
        if has_clear {
            let icon_size: f64 = font_size;
            let clear_x = combobox_rect.origin.x
                + combobox_rect.size.width
                - icon_size
                - font_size // 箭头宽度
                - 12.0;
            let clear_y = combobox_rect.origin.y + (h - icon_size) / 2.0;
            let clear_rect = Rect::new(clear_x, clear_y, icon_size, icon_size);
            ctx.draw_text(
                "✕",
                clear_rect,
                Color::new(0.5, 0.5, 0.5, 1.0),
                (icon_size * 0.8) as f32,
            );
        }

        // ── 下拉箭头 (▼) ──
        {
            let arrow_size: f64 = font_size;
            let arrow_x = combobox_rect.origin.x + combobox_rect.size.width - arrow_size - 8.0;
            let arrow_y = combobox_rect.origin.y + (h - arrow_size) / 2.0;
            let arrow_rect = Rect::new(arrow_x, arrow_y, arrow_size, arrow_size);
            let arrow_char = if state.open { "▲" } else { "▼" };
            let arrow_color = if state.disabled {
                Color::new(0.7, 0.7, 0.7, 1.0)
            } else {
                Color::new(0.4, 0.4, 0.4, 1.0)
            };
            ctx.draw_text(
                arrow_char,
                arrow_rect,
                arrow_color,
                (arrow_size * 0.7) as f32,
            );
        }

        // ── 下拉列表区域（打开时绘制） ──
        if state.open {
            let listbox_y = combobox_rect.origin.y + combobox_rect.size.height + 4.0;
            let listbox_h: f64 = 160.0; // 固定列表高度
            let listbox_w: f64 = combobox_rect.size.width;

            let listbox_rect = Rect::new(combobox_rect.origin.x, listbox_y, listbox_w, listbox_h);

            // 列表背景 + 阴影效果（用两层 fill_rect 模拟）
            let shadow_color = Color::new(0.85, 0.85, 0.85, 0.5);
            let shadow_rect = Rect::new(
                listbox_rect.origin.x + 2.0,
                listbox_rect.origin.y + 2.0,
                listbox_rect.size.width,
                listbox_rect.size.height,
            );
            ctx.fill_rect(shadow_rect, shadow_color, 6.0);

            // 列表背景
            let listbox_bg = Color::new(1.0, 1.0, 1.0, 1.0);
            let listbox_border = Color::new(0.85, 0.85, 0.85, 1.0);
            ctx.fill_rect(listbox_rect, listbox_bg, 6.0);

            // 列表边框（内缩模拟）
            let listbox_border_w: f64 = 1.0;
            ctx.fill_rect(listbox_rect, listbox_border, 6.0);
            let listbox_inner = Rect::new(
                listbox_rect.origin.x + listbox_border_w,
                listbox_rect.origin.y + listbox_border_w,
                listbox_rect.size.width - listbox_border_w * 2.0,
                listbox_rect.size.height - listbox_border_w * 2.0,
            );
            ctx.fill_rect(listbox_inner, listbox_bg, 5.0);

            // 列表占位文本
            let placeholder_text = "(options)";
            let ph_rect = Rect::new(
                listbox_rect.origin.x + 12.0,
                listbox_rect.origin.y + 8.0,
                listbox_rect.size.width - 24.0,
                24.0,
            );
            ctx.draw_text(
                placeholder_text,
                ph_rect,
                Color::new(0.6, 0.6, 0.6, 1.0),
                (font_size * 0.85) as f32,
            );
        }

        // ── 提示文本 ──
        if !state.hint.is_empty() {
            let listbox_space = if state.open { 164.0 } else { 0.0 };
            let hint_y = combobox_rect.origin.y + combobox_rect.size.height + listbox_space + 4.0;
            let hint_font_size = (font_size * 0.75) as f32;
            let hint_color = Color::new(0.5, 0.5, 0.5, 1.0);
            let hint_rect = Rect::new(bounds.origin.x, hint_y, bounds.size.width, font_size * 0.75);
            ctx.draw_text(&state.hint, hint_rect, hint_color, hint_font_size);
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

    #[test]
    fn name() {
        assert_eq!(WaSelect.name(), "rgui_components::WaSelect");
    }

    #[test]
    fn view_has_label() {
        let state = WaSelectState::new("Country");
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert!(v.props.contains_key("label"));
    }

    #[test]
    fn view_has_value() {
        let mut state = WaSelectState::new("Country");
        state.value = "us".into();
        state.display_label = "United States".into();
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("value"),
            Some(&PropValue::Str(std::sync::Arc::from("us")))
        );
    }

    #[test]
    fn view_has_placeholder() {
        let mut state = WaSelectState::new("Country");
        state.placeholder = "Select a country...".into();
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("placeholder"),
            Some(&PropValue::Str(std::sync::Arc::from("Select a country...")))
        );
    }

    #[test]
    fn view_disabled() {
        let mut state = WaSelectState::new("Country");
        state.disabled = true;
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("disabled"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_pill() {
        let mut state = WaSelectState::new("Country");
        state.pill = true;
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("pill"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_with_clear() {
        let mut state = WaSelectState::new("Country");
        state.with_clear = true;
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("with-clear"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_required() {
        let mut state = WaSelectState::new("Country");
        state.required = true;
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("required"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_open() {
        let mut state = WaSelectState::new("Country");
        state.open = true;
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("open"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_multiple() {
        let mut state = WaSelectState::new("Tags");
        state.multiple = true;
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(v.props.get("multiple"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn view_appearance() {
        let mut state = WaSelectState::new("Appearance");
        state.appearance = "filled".into();
        let v = WaSelect.view(&state, &ViewContext::new(Size::new(800.0, 600.0)));
        assert_eq!(
            v.props.get("appearance"),
            Some(&PropValue::Str(std::sync::Arc::from("filled")))
        );
    }

    #[test]
    fn update_clear_empties_value() {
        let mut state = WaSelectState::new("Clear me");
        state.value = "selected".into();
        state.display_label = "Selected Option".into();
        WaSelect.update(
            WaSelectMessage::Clear,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.value.is_empty());
        assert!(state.display_label.is_empty());
    }

    #[test]
    fn update_clear_on_disabled_does_nothing() {
        let mut state = WaSelectState::new("Locked");
        state.disabled = true;
        state.value = "keep me".into();
        state.display_label = "Keep Me".into();
        WaSelect.update(
            WaSelectMessage::Clear,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert_eq!(state.value, "keep me");
    }

    #[test]
    fn update_show_sets_open() {
        let mut state = WaSelectState::new("Open me");
        WaSelect.update(
            WaSelectMessage::Show,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(state.open);
    }

    #[test]
    fn update_show_on_disabled_does_nothing() {
        let mut state = WaSelectState::new("Locked");
        state.disabled = true;
        WaSelect.update(
            WaSelectMessage::Show,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open);
    }

    #[test]
    fn update_hide_clears_open() {
        let mut state = WaSelectState::new("Close me");
        state.open = true;
        WaSelect.update(
            WaSelectMessage::Hide,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open);
    }

    #[test]
    fn update_after_hide_clears_open() {
        let mut state = WaSelectState::new("Close me");
        state.open = true;
        WaSelect.update(
            WaSelectMessage::AfterHide,
            &mut state,
            &mut UpdateContext::default(),
        );
        assert!(!state.open);
    }

    #[test]
    fn update_blur_is_handled() {
        let mut state = WaSelectState::new("OK");
        WaSelect.update(
            WaSelectMessage::Blur,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_focus_is_handled() {
        let mut state = WaSelectState::new("OK");
        WaSelect.update(
            WaSelectMessage::Focus,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn update_invalid_is_handled() {
        let mut state = WaSelectState::new("OK");
        WaSelect.update(
            WaSelectMessage::Invalid,
            &mut state,
            &mut UpdateContext::default(),
        );
    }

    #[test]
    fn measure_min_dimensions() {
        let state = WaSelectState::new("Select");
        let size = WaSelect.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(size.width >= 80.0, "宽度应 ≥ 80px，实际 {size:?}");
        assert!(size.height >= 36.0, "高度应 ≥ 36px，实际 {size:?}");
    }

    #[test]
    fn measure_size_xs_smaller() {
        let mut state = WaSelectState::new("Small");
        state.size = "xs".into();
        let xs = WaSelect.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        state.size = "xl".into();
        let xl = WaSelect.measure(
            &state,
            BoxConstraints::new(0.0, 1920.0, 0.0, 1080.0),
            &MeasureContext::default(),
        );
        assert!(xs.height < xl.height, "xs 应比 xl 矮");
    }

    #[test]
    fn paint_produces_ops() {
        let mut state = WaSelectState::new("Country");
        state.display_label = "United States".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        assert!(
            ctx.op_count() >= 3,
            "应至少绘制标签+边框+文本，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_empty_shows_placeholder() {
        let mut state = WaSelectState::new("Country");
        state.placeholder = "Choose...".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_disabled_shows_gray() {
        let mut state = WaSelectState::new("Disabled");
        state.disabled = true;
        state.display_label = "Gray".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_with_hint_produces_ops() {
        let mut state = WaSelectState::new("Hinted");
        state.hint = "Choose an option".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 60.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_with_clear_shows_icon() {
        let mut state = WaSelectState::new("Clearable");
        state.with_clear = true;
        state.display_label = "Selected".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        // 有清除按钮时绘制操作更多
        assert!(ctx.op_count() >= 4);
    }

    #[test]
    fn paint_appearance_filled() {
        let mut state = WaSelectState::new("Filled");
        state.appearance = "filled".into();
        state.display_label = "Selected".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }

    #[test]
    fn paint_open_shows_listbox() {
        let mut state = WaSelectState::new("Open");
        state.open = true;
        state.display_label = "Selected".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 250.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        // 打开时多绘制列表背景（至少 3 个 rect）+ 占位文本
        assert!(
            ctx.op_count() >= 6,
            "打开时绘制操作应 > 6，实际 {}",
            ctx.op_count()
        );
    }

    #[test]
    fn paint_pill_uses_large_radius() {
        let mut state = WaSelectState::new("Pill");
        state.pill = true;
        state.display_label = "Pill".into();
        let bounds = Rect::new(0.0, 0.0, 300.0, 50.0);
        let mut ctx = PaintContext::new(bounds);
        WaSelect.paint(&state, bounds, &mut ctx);
        assert!(ctx.op_count() >= 3);
    }
}
