//! PaintFn 工厂——根据 WidgetView.props 分发到对应组件的 paint() 方法。
//!
//! 组件通过 `wa-translate` 技能从 Web Awesome (MIT) 手工翻译加入
//! `rgui-components`。翻译后在此添加对应的 match 分支。
//!
//! 布局容器（Container/Row/Column 等）自身不绘制，
//! 子节点的 paint 结果由 walk_view_tree 递归收集。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::traits::AppMessage;
use rgui_core::view::PropValue;
use rgui_render::PaintFn;

/// 创建默认的 PaintFn。
///
/// 调度到已翻译组件的 paint() 方法。
/// 未翻译类型返回空 Vec。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
    fn get_str<'a>(
        props: &'a std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
        key: &str,
    ) -> Option<&'a str> {
        match props.get(key) {
            Some(rgui_core::view::PropValue::Str(s)) => Some(s),
            _ => None,
        }
    }

    Box::new(
        move |view: &rgui_core::view::WidgetView<M>, bounds: Rect| -> Vec<PaintOp> {
            match view.widget_type {
                // ── WA 翻译组件 ──
                "WaBadge" => {
                    use rgui_components::wa_badge::{WaBadge, WaBadgeState};
                    let mut state = WaBadgeState::new();
                    if let Some(v) = get_str(&view.props, "variant") {
                        state.variant = v.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
                    }
                    if let Some(pill) = view.props.get("pill") {
                        if let PropValue::Bool(b) = pill {
                            state.pill = *b;
                        }
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaBadge, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaButton" => {
                    use rgui_components::wa_button::{WaButton, WaButtonState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let state = WaButtonState::new(label);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaButton, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaDivider" => {
                    use rgui_components::wa_divider::{WaDivider, WaDividerState};
                    let orientation = get_str(&view.props, "orientation").unwrap_or("horizontal");
                    let state = WaDividerState::new(orientation);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaDivider, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaInput" => {
                    use rgui_components::wa_input::{WaInput, WaInputState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaInputState::new(label);
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
                    }
                    if let Some(t) = get_str(&view.props, "type") {
                        state.r#type = t.to_string();
                    }
                    if let Some(v) = get_str(&view.props, "value") {
                        state.value = v.to_string();
                    }
                    if let Some(p) = get_str(&view.props, "placeholder") {
                        state.placeholder = p.to_string();
                    }
                    if let Some(h) = get_str(&view.props, "hint") {
                        state.hint = h.to_string();
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("readonly") {
                        state.readonly = *r;
                    }
                    if let Some(PropValue::Bool(rq)) = view.props.get("required") {
                        state.required = *rq;
                    }
                    if let Some(PropValue::Bool(b)) = view.props.get("pill") {
                        state.pill = *b;
                    }
                    if let Some(PropValue::Bool(b)) = view.props.get("with-clear") {
                        state.with_clear = *b;
                    }
                    if let Some(PropValue::Bool(b)) = view.props.get("password-toggle") {
                        state.password_toggle = *b;
                    }
                    if let Some(PropValue::Bool(b)) = view.props.get("password-visible") {
                        state.password_visible = *b;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaInput, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaCard" => {
                    use rgui_components::wa_card::{WaCard, WaCardState};
                    let mut state = WaCardState::new();
                    if let Some(app) = get_str(&view.props, "appearance") {
                        state.appearance = app.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaCard, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaCheckbox" => {
                    use rgui_components::wa_checkbox::{WaCheckbox, WaCheckboxState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaCheckboxState::new(label);
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Bool(c)) = view.props.get("checked") {
                        state.checked = *c;
                    }
                    if let Some(PropValue::Bool(i)) = view.props.get("indeterminate") {
                        state.indeterminate = *i;
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("required") {
                        state.required = *r;
                    }
                    if let Some(h) = get_str(&view.props, "hint") {
                        state.hint = h.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaCheckbox, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaCheckboxGroup" => {
                    use rgui_components::wa_checkbox_group::{WaCheckboxGroup, WaCheckboxGroupState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaCheckboxGroupState::new(label);
                    if let Some(h) = get_str(&view.props, "hint") {
                        state.hint = h.to_string();
                    }
                    if let Some(o) = get_str(&view.props, "orientation") {
                        state.orientation = o.to_string();
                    }
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("required") {
                        state.required = *r;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaCheckboxGroup, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaRadio" => {
                    use rgui_components::wa_radio::{WaRadio, WaRadioState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaRadioState::new(label);
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Bool(c)) = view.props.get("checked") {
                        state.checked = *c;
                    }
                    if let Some(v) = get_str(&view.props, "value") {
                        state.value = v.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaRadio, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaRadioGroup" => {
                    use rgui_components::wa_radio_group::{WaRadioGroup, WaRadioGroupState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaRadioGroupState::new(label);
                    if let Some(h) = get_str(&view.props, "hint") {
                        state.hint = h.to_string();
                    }
                    if let Some(o) = get_str(&view.props, "orientation") {
                        state.orientation = o.to_string();
                    }
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("required") {
                        state.required = *r;
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(v) = get_str(&view.props, "value") {
                        state.value = v.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaRadioGroup, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaSwitch" => {
                    use rgui_components::wa_switch::{WaSwitch, WaSwitchState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaSwitchState::new(label);
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Bool(c)) = view.props.get("checked") {
                        state.checked = *c;
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("required") {
                        state.required = *r;
                    }
                    if let Some(h) = get_str(&view.props, "hint") {
                        state.hint = h.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaSwitch, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaDetails" => {
                    use rgui_components::wa_details::{WaDetails, WaDetailsState};
                    let mut state = WaDetailsState::new();
                    if let Some(open) = view.props.get("open") {
                        if let PropValue::Bool(b) = open {
                            state.open = *b;
                        }
                    }
                    if let Some(summary) = get_str(&view.props, "summary") {
                        state.summary = summary.to_string();
                    }
                    if let Some(app) = get_str(&view.props, "appearance") {
                        state.appearance = app.to_string();
                    }
                    if let Some(ip) = get_str(&view.props, "icon_placement") {
                        state.icon_placement = ip.to_string();
                    }
                    if let Some(disabled) = view.props.get("disabled") {
                        if let PropValue::Bool(b) = disabled {
                            state.disabled = *b;
                        }
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaDetails, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                "WaIcon" => {
                    use rgui_components::wa_icon::{WaIcon, WaIconState};
                    let name = get_str(&view.props, "name").unwrap_or("");
                    let mut state = WaIconState::new(name);
                    if let Some(label) = get_str(&view.props, "label") {
                        state.label = label.to_string();
                    }
                    if let Some(size) = get_str(&view.props, "size") {
                        state.size = size.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaIcon, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaAvatar" => {
                    use rgui_components::wa_avatar::{WaAvatar, WaAvatarState};
                    let state = WaAvatarState::new();
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaAvatar, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaBreadcrumb" => {
                    use rgui_components::wa_breadcrumb::{WaBreadcrumb, WaBreadcrumbState};
                    let mut state = WaBreadcrumbState::new();
                    if let Some(label) = get_str(&view.props, "label") {
                        state.label = label.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaBreadcrumb, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaBreadcrumbItem" => {
                    use rgui_components::wa_breadcrumb_item::{WaBreadcrumbItem, WaBreadcrumbItemState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaBreadcrumbItemState::new(label);
                    if let Some(href) = get_str(&view.props, "href") {
                        state.href = href.to_string();
                    }
                    if let Some(sep) = view.props.get("separator") {
                        if let PropValue::Bool(b) = sep {
                            state.separator = *b;
                        }
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaBreadcrumbItem, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaSpinner" => {
                    use rgui_components::wa_spinner::{WaSpinner, WaSpinnerState};
                    let state = WaSpinnerState::new();
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaSpinner, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaCopyButton" => {
                    use rgui_components::wa_copy_button::{WaCopyButton, WaCopyButtonState};
                    let mut state = WaCopyButtonState::new("");
                    if let Some(v) = get_str(&view.props, "value") {
                        state.value = v.to_string();
                    }
                    if let Some(l) = get_str(&view.props, "copy_label") {
                        state.copy_label = l.to_string();
                    }
                    if let Some(l) = get_str(&view.props, "success_label") {
                        state.success_label = l.to_string();
                    }
                    if let Some(l) = get_str(&view.props, "error_label") {
                        state.error_label = l.to_string();
                    }
                    if let Some(disabled) = view.props.get("disabled") {
                        if let PropValue::Bool(b) = disabled {
                            state.disabled = *b;
                        }
                    }
                    if let Some(s) = get_str(&view.props, "status") {
                        state.status = s.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaCopyButton, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                // ── 布局容器（自身不绘制）──
                "Container" | "Row" | "Column" | "Padding" | "Center" | "Expanded" | "SizedBox"
                | "Card" | "Stack" | "ScrollView" | "ListView" => Vec::new(),

                // ── 未翻译 / 未知 ──
                unknown => {
                    eprintln!(
                        "[rgui] paint_factory: 未知 widget_type=\"{unknown}\"，无 paint 实现，返回空"
                    );
                    Vec::new()
                },
            }
        },
    )
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::geometry::Rect;
    use rgui_core::view::{PropValue, WidgetView};

    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Dummy,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            "dummy"
        }
    }

    #[test]
    fn default_paint_fn_creates_valid_paint_fn() {
        let _paint_fn = default_paint_fn::<TestMsg>();
    }

    #[test]
    fn wa_button_paint_produces_ops() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = WidgetView::<TestMsg>::new("WaButton").prop("label", PropValue::str("Click"));
        let ops = paint_fn(&view, Rect::new(0.0, 0.0, 120.0, 40.0));
        assert!(!ops.is_empty(), "WaButton 应产生绘制操作");
    }

    #[test]
    fn unknown_type_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = WidgetView::<TestMsg>::new("Unknown");
        let ops = paint_fn(&view, Rect::new(0.0, 0.0, 100.0, 40.0));
        assert!(ops.is_empty());
    }
}
