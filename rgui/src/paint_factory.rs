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

use crate::widget_state::WidgetStateStore;

/// 内部实现：PaintFn 工厂（共享 match 体，避免代码重复）。
///
/// 当 `store` 为 `Some` 时，WaAccordionItem 从持久存储读取状态；
/// 为 `None` 时，所有组件从 WidgetView.props 创建临时状态。
#[must_use]
fn paint_fn_impl<M: AppMessage>(store: Option<WidgetStateStore>) -> PaintFn<M> {
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
                "WaTextarea" => {
                    use rgui_components::wa_textarea::{WaTextarea, WaTextareaState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaTextareaState::new(label);
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
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
                    if let Some(n) = get_str(&view.props, "name") {
                        state.name = n.to_string();
                    }
                    if let Some(r) = get_str(&view.props, "resize") {
                        state.resize = r.to_string();
                    }
                    if let Some(im) = get_str(&view.props, "inputmode") {
                        state.inputmode = im.to_string();
                    }
                    if let Some(PropValue::Int(r)) = view.props.get("rows") {
                        state.rows = *r as u32;
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
                    if let Some(PropValue::Bool(s)) = view.props.get("spellcheck") {
                        state.spellcheck = *s;
                    }
                    if let Some(PropValue::Bool(wc)) = view.props.get("with-count") {
                        state.with_count = *wc;
                    }
                    if let Some(PropValue::Int(ml)) = view.props.get("minlength") {
                        state.minlength = Some(*ml as u32);
                    }
                    if let Some(PropValue::Int(ml)) = view.props.get("maxlength") {
                        state.maxlength = Some(*ml as u32);
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaTextarea, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaSlider" => {
                    use rgui_components::wa_slider::{WaSlider, WaSliderState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaSliderState::new(label);
                    if let Some(PropValue::Float(v)) = view.props.get("value") {
                        state.value = v.0;
                    }
                    if let Some(PropValue::Float(m)) = view.props.get("min") {
                        state.min = m.0;
                    }
                    if let Some(PropValue::Float(m)) = view.props.get("max") {
                        state.max = m.0;
                    }
                    if let Some(PropValue::Float(s)) = view.props.get("step") {
                        state.step = s.0;
                    }
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(o) = get_str(&view.props, "orientation") {
                        state.orientation = o.to_string();
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("readonly") {
                        state.readonly = *r;
                    }
                    if let Some(h) = get_str(&view.props, "hint") {
                        state.hint = h.to_string();
                    }
                    if let Some(PropValue::Bool(wm)) = view.props.get("with-markers") {
                        state.with_markers = *wm;
                    }
                    if let Some(PropValue::Float(o)) = view.props.get("indicator-offset") {
                        state.indicator_offset = Some(o.0);
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaSlider, &state, bounds, &mut ctx);
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
                "WaSelect" => {
                    use rgui_components::wa_select::{WaSelect, WaSelectState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaSelectState::new(label);
                    if let Some(n) = get_str(&view.props, "name") {
                        state.name = n.to_string();
                    }
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
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
                    if let Some(dl) = get_str(&view.props, "display-label") {
                        state.display_label = dl.to_string();
                    }
                    if let Some(pl) = get_str(&view.props, "placement") {
                        state.placement = pl.to_string();
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Bool(m)) = view.props.get("multiple") {
                        state.multiple = *m;
                    }
                    if let Some(PropValue::Bool(p)) = view.props.get("pill") {
                        state.pill = *p;
                    }
                    if let Some(PropValue::Bool(w)) = view.props.get("with-clear") {
                        state.with_clear = *w;
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("required") {
                        state.required = *r;
                    }
                    if let Some(PropValue::Bool(o)) = view.props.get("open") {
                        state.open = *o;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaSelect, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaSkeleton" => {
                    use rgui_components::wa_skeleton::{WaSkeleton, WaSkeletonState};
                    let mut state = WaSkeletonState::new();
                    if let Some(effect) = get_str(&view.props, "effect") {
                        state.effect = effect.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaSkeleton, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaSplitPanel" => {
                    use rgui_components::wa_split_panel::{WaSplitPanel, WaSplitPanelState};
                    let mut state = WaSplitPanelState::new();
                    if let Some(orientation) = get_str(&view.props, "orientation") {
                        state.orientation = orientation.to_string();
                    }
                    if let Some(PropValue::Float(v)) = view.props.get("position") {
                        state.position = v.0;
                    }
                    if let Some(PropValue::Bool(b)) = view.props.get("disabled") {
                        state.disabled = *b;
                    }
                    if let Some(PropValue::Str(s)) = view.props.get("primary") {
                        state.primary = Some(s.to_string());
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaSplitPanel, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaComparison" => {
                    use rgui_components::wa_comparison::{WaComparison, WaComparisonState};
                    let mut state = WaComparisonState::new();
                    if let Some(PropValue::Float(v)) = view.props.get("position") {
                        state.position = v.0;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaComparison, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaDialog" => {
                    use rgui_components::wa_dialog::{WaDialog, WaDialogState};
                    let mut state = WaDialogState::new();
                    if let Some(l) = get_str(&view.props, "label") {
                        state.label = l.to_string();
                    }
                    if let Some(PropValue::Bool(o)) = view.props.get("open") {
                        state.open = *o;
                    }
                    if let Some(PropValue::Bool(w)) = view.props.get("without-header") {
                        state.without_header = *w;
                    }
                    if let Some(PropValue::Bool(ld)) = view.props.get("light-dismiss") {
                        state.light_dismiss = *ld;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaDialog, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaDrawer" => {
                    use rgui_components::wa_drawer::{WaDrawer, WaDrawerState};
                    let mut state = WaDrawerState::new();
                    if let Some(l) = get_str(&view.props, "label") {
                        state.label = l.to_string();
                    }
                    if let Some(p) = get_str(&view.props, "placement") {
                        state.placement = p.to_string();
                    }
                    if let Some(PropValue::Bool(o)) = view.props.get("open") {
                        state.open = *o;
                    }
                    if let Some(PropValue::Bool(w)) = view.props.get("without-header") {
                        state.without_header = *w;
                    }
                    if let Some(PropValue::Bool(ld)) = view.props.get("light-dismiss") {
                        state.light_dismiss = *ld;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaDrawer, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaDropdown" => {
                    use rgui_components::wa_dropdown::{WaDropdown, WaDropdownState};
                    let mut state = WaDropdownState::new();
                    if let Some(p) = get_str(&view.props, "placement") {
                        state.placement = p.to_string();
                    }
                    if let Some(PropValue::Bool(o)) = view.props.get("open") {
                        state.open = *o;
                    }
                    if let Some(PropValue::Float(d)) = view.props.get("distance") {
                        state.distance = d.0;
                    }
                    if let Some(PropValue::Float(s)) = view.props.get("skidding") {
                        state.skidding = s.0;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaDropdown, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaDropdownItem" => {
                    use rgui_components::wa_dropdown_item::{WaDropdownItem, WaDropdownItemState};
                    let mut state = WaDropdownItemState::new();
                    if let Some(PropValue::Bool(a)) = view.props.get("active") {
                        state.active = *a;
                    }
                    if let Some(s) = get_str(&view.props, "variant") {
                        state.variant = s.to_string();
                    }
                    if let Some(s) = get_str(&view.props, "value") {
                        state.value = s.to_string();
                    }
                    // label 作为显示文本（覆盖 value 的显示用途）
                    if let Some(s) = get_str(&view.props, "label") {
                        state.value = s.to_string();
                    }
                    if let Some(s) = get_str(&view.props, "type") {
                        state.type_ = s.to_string();
                    }
                    if let Some(PropValue::Bool(c)) = view.props.get("checked") {
                        state.checked = *c;
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaDropdownItem, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaPopup" => {
                    use rgui_components::wa_popup::{WaPopup, WaPopupState};
                    let mut state = WaPopupState::new();
                    if let Some(PropValue::Bool(a)) = view.props.get("active") {
                        state.active = *a;
                    }
                    if let Some(p) = get_str(&view.props, "placement") {
                        state.placement = p.to_string();
                    }
                    if let Some(PropValue::Float(d)) = view.props.get("distance") {
                        state.distance = d.0;
                    }
                    if let Some(PropValue::Float(s)) = view.props.get("skidding") {
                        state.skidding = s.0;
                    }
                    if let Some(PropValue::Bool(a)) = view.props.get("arrow") {
                        state.arrow = *a;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaPopup, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaPopover" => {
                    use rgui_components::wa_popover::{WaPopover, WaPopoverState};
                    let mut state = WaPopoverState::new();
                    if let Some(p) = get_str(&view.props, "placement") {
                        state.placement = p.to_string();
                    }
                    if let Some(PropValue::Bool(o)) = view.props.get("open") {
                        state.open = *o;
                    }
                    if let Some(PropValue::Float(d)) = view.props.get("distance") {
                        state.distance = d.0;
                    }
                    if let Some(PropValue::Float(s)) = view.props.get("skidding") {
                        state.skidding = s.0;
                    }
                    if let Some(PropValue::Bool(w)) = view.props.get("without-arrow") {
                        state.without_arrow = *w;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaPopover, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaTooltip" => {
                    use rgui_components::wa_tooltip::{WaTooltip, WaTooltipState};
                    let mut state = WaTooltipState::new();
                    if let Some(p) = get_str(&view.props, "placement") {
                        state.placement = p.to_string();
                    }
                    if let Some(PropValue::Bool(o)) = view.props.get("open") {
                        state.open = *o;
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Float(d)) = view.props.get("distance") {
                        state.distance = d.0;
                    }
                    if let Some(PropValue::Float(s)) = view.props.get("skidding") {
                        state.skidding = s.0;
                    }
                    if let Some(PropValue::Float(sd)) = view.props.get("show-delay") {
                        state.show_delay = sd.0;
                    }
                    if let Some(PropValue::Float(hd)) = view.props.get("hide-delay") {
                        state.hide_delay = hd.0;
                    }
                    if let Some(t) = get_str(&view.props, "trigger") {
                        state.trigger = t.to_string();
                    }
                    if let Some(PropValue::Bool(w)) = view.props.get("without-arrow") {
                        state.without_arrow = *w;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaTooltip, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaProgressBar" => {
                    use rgui_components::wa_progress_bar::{WaProgressBar, WaProgressBarState};
                    let mut state = WaProgressBarState::new();
                    if let Some(PropValue::Float(v)) = view.props.get("value") {
                        state.value = v.0;
                    }
                    if let Some(PropValue::Bool(b)) = view.props.get("indeterminate") {
                        state.indeterminate = *b;
                    }
                    if let Some(PropValue::Str(s)) = view.props.get("label") {
                        state.label = s.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaProgressBar, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaProgressRing" => {
                    use rgui_components::wa_progress_ring::{WaProgressRing, WaProgressRingState};
                    let mut state = WaProgressRingState::new();
                    if let Some(PropValue::Float(v)) = view.props.get("value") {
                        state.value = v.0;
                    }
                    if let Some(PropValue::Str(s)) = view.props.get("label") {
                        state.label = s.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaProgressRing, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaRating" => {
                    use rgui_components::wa_rating::{WaRating, WaRatingState};
                    let mut state = WaRatingState::new();
                    if let Some(PropValue::Float(v)) = view.props.get("value") {
                        state.value = v.0;
                    }
                    if let Some(PropValue::Float(m)) = view.props.get("max") {
                        state.max = m.0;
                    }
                    if let Some(PropValue::Float(p)) = view.props.get("precision") {
                        state.precision = p.0;
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
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(l) = get_str(&view.props, "label") {
                        state.label = l.to_string();
                    }
                    if let Some(n) = get_str(&view.props, "name") {
                        state.name = n.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaRating, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaTab" => {
                    use rgui_components::wa_tab::{WaTab, WaTabState};
                    let mut state = WaTabState::new();
                    if let Some(p) = get_str(&view.props, "panel") {
                        state.panel = p.to_string();
                    }
                    if let Some(l) = get_str(&view.props, "label") {
                        state.label = l.to_string();
                    }
                    if let Some(PropValue::Bool(a)) = view.props.get("active") {
                        state.active = *a;
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaTab, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaTabGroup" => {
                    use rgui_components::wa_tab_group::{WaTabGroup, WaTabGroupState};
                    let mut state = WaTabGroupState::new();
                    if let Some(a) = get_str(&view.props, "active") {
                        state.active = a.to_string();
                    }
                    if let Some(p) = get_str(&view.props, "placement") {
                        state.placement = p.to_string();
                    }
                    if let Some(act) = get_str(&view.props, "activation") {
                        state.activation = act.to_string();
                    }
                    if let Some(PropValue::Bool(w)) = view.props.get("without-scroll-controls") {
                        state.without_scroll_controls = *w;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaTabGroup, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaTabPanel" => {
                    use rgui_components::wa_tab_panel::{WaTabPanel, WaTabPanelState};
                    let mut state = WaTabPanelState::new();
                    if let Some(n) = get_str(&view.props, "name") {
                        state.name = n.to_string();
                    }
                    if let Some(PropValue::Bool(a)) = view.props.get("active") {
                        state.active = *a;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaTabPanel, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaColorPicker" => {
                    use rgui_components::wa_color_picker::{WaColorPicker, WaColorPickerState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaColorPickerState::new(label);
                    if let Some(n) = get_str(&view.props, "name") {
                        state.name = n.to_string();
                    }
                    if let Some(v) = get_str(&view.props, "value") {
                        state.value = v.to_string();
                        state.is_empty = v.is_empty();
                    }
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(f) = get_str(&view.props, "format") {
                        state.format = f.to_string();
                    }
                    if let Some(h) = get_str(&view.props, "hint") {
                        state.hint = h.to_string();
                    }
                    if let Some(PropValue::Bool(d)) = view.props.get("disabled") {
                        state.disabled = *d;
                    }
                    if let Some(PropValue::Bool(o)) = view.props.get("open") {
                        state.open = *o;
                    }
                    if let Some(PropValue::Bool(u)) = view.props.get("uppercase") {
                        state.uppercase = *u;
                    }
                    if let Some(PropValue::Bool(w)) = view.props.get("without-format-toggle") {
                        state.without_format_toggle = *w;
                    }
                    if let Some(PropValue::Bool(r)) = view.props.get("required") {
                        state.required = *r;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaColorPicker, &state, bounds, &mut ctx);
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
                    use rgui_components::wa_checkbox_group::{
                        WaCheckboxGroup, WaCheckboxGroupState,
                    };
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
                    rgui_core::traits::WidgetSpec::paint(
                        &WaCheckboxGroup,
                        &state,
                        bounds,
                        &mut ctx,
                    );
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
                    use rgui_components::wa_breadcrumb_item::{
                        WaBreadcrumbItem, WaBreadcrumbItemState,
                    };
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
                    rgui_core::traits::WidgetSpec::paint(
                        &WaBreadcrumbItem,
                        &state,
                        bounds,
                        &mut ctx,
                    );
                    ctx.into_operations()
                },
                "WaSpinner" => {
                    use rgui_components::wa_spinner::{WaSpinner, WaSpinnerState};
                    let state = WaSpinnerState::new();
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaSpinner, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaCallout" => {
                    use rgui_components::wa_callout::{WaCallout, WaCalloutState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaCalloutState::new(label);
                    if let Some(v) = get_str(&view.props, "variant") {
                        state.variant = v.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
                    }
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaCallout, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaTag" => {
                    use rgui_components::wa_tag::{WaTag, WaTagState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let mut state = WaTagState::new(label);
                    if let Some(v) = get_str(&view.props, "variant") {
                        state.variant = v.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
                    }
                    if let Some(sz) = get_str(&view.props, "size") {
                        state.size = sz.to_string();
                    }
                    if let Some(pill) = view.props.get("pill") {
                        if let PropValue::Bool(b) = pill {
                            state.pill = *b;
                        }
                    }
                    if let Some(wr) = view.props.get("with_remove") {
                        if let PropValue::Bool(b) = wr {
                            state.with_remove = *b;
                        }
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaTag, &state, bounds, &mut ctx);
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
                "WaAccordionItem" => {
                    use rgui_components::wa_accordion_item::{
                        WaAccordionItem, WaAccordionItemState,
                    };

                    // 若有持久存储则读已有状态；否则每帧从 props 创建
                    let state = if let Some(ref store) = store {
                        let widget_id = view.id.unwrap_or_default();
                        store.get_or_init(widget_id, || {
                            let mut s = WaAccordionItemState::new();
                            if let Some(l) = get_str(&view.props, "label") {
                                s.label = l.to_string();
                            }
                            if let Some(expanded) = view.props.get("expanded") {
                                if let PropValue::Bool(b) = expanded {
                                    s.expanded = *b;
                                }
                            }
                            if let Some(disabled) = view.props.get("disabled") {
                                if let PropValue::Bool(b) = disabled {
                                    s.disabled = *b;
                                }
                            }
                            if let Some(ip) = get_str(&view.props, "icon-placement") {
                                s.icon_placement = ip.to_string();
                            }
                            if let Some(a) = get_str(&view.props, "appearance") {
                                s.appearance = a.to_string();
                            }
                            if let Some(hl) = get_str(&view.props, "heading-level") {
                                s.heading_level = hl.to_string();
                            }
                            if let Some(c) = get_str(&view.props, "content") {
                                s.content = c.to_string();
                            }
                            s
                        })
                    } else {
                        let mut s = WaAccordionItemState::new();
                        if let Some(l) = get_str(&view.props, "label") {
                            s.label = l.to_string();
                        }
                        if let Some(expanded) = view.props.get("expanded") {
                            if let PropValue::Bool(b) = expanded {
                                s.expanded = *b;
                            }
                        }
                        if let Some(disabled) = view.props.get("disabled") {
                            if let PropValue::Bool(b) = disabled {
                                s.disabled = *b;
                            }
                        }
                        if let Some(ip) = get_str(&view.props, "icon-placement") {
                            s.icon_placement = ip.to_string();
                        }
                        if let Some(a) = get_str(&view.props, "appearance") {
                            s.appearance = a.to_string();
                        }
                        if let Some(hl) = get_str(&view.props, "heading-level") {
                            s.heading_level = hl.to_string();
                        }
                        if let Some(c) = get_str(&view.props, "content") {
                            s.content = c.to_string();
                        }
                        s
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(
                        &WaAccordionItem,
                        &state,
                        bounds,
                        &mut ctx,
                    );
                    ctx.into_operations()
                },

                "WaFormatBytes" => {
                    use rgui_components::wa_format_bytes::{WaFormatBytes, WaFormatBytesState};
                    let mut state = WaFormatBytesState::new();
                    if let Some(PropValue::Float(v)) = view.props.get("value") {
                        state.value = v.0;
                    }
                    if let Some(u) = get_str(&view.props, "unit") {
                        state.unit = u.to_string();
                    }
                    if let Some(d) = get_str(&view.props, "display") {
                        state.display = d.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaFormatBytes, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                "WaFormatDate" => {
                    use rgui_components::wa_format_date::{WaFormatDate, WaFormatDateState};
                    let mut state = WaFormatDateState::new();
                    if let Some(d) = get_str(&view.props, "date") {
                        state.date = d.to_string();
                    }
                    if let Some(w) = get_str(&view.props, "weekday") {
                        state.weekday = w.to_string();
                    }
                    if let Some(e) = get_str(&view.props, "era") {
                        state.era = e.to_string();
                    }
                    if let Some(y) = get_str(&view.props, "year") {
                        state.year = y.to_string();
                    }
                    if let Some(m) = get_str(&view.props, "month") {
                        state.month = m.to_string();
                    }
                    if let Some(d) = get_str(&view.props, "day") {
                        state.day = d.to_string();
                    }
                    if let Some(h) = get_str(&view.props, "hour") {
                        state.hour = h.to_string();
                    }
                    if let Some(mi) = get_str(&view.props, "minute") {
                        state.minute = mi.to_string();
                    }
                    if let Some(s) = get_str(&view.props, "second") {
                        state.second = s.to_string();
                    }
                    if let Some(tzn) = get_str(&view.props, "time-zone-name") {
                        state.time_zone_name = tzn.to_string();
                    }
                    if let Some(tz) = get_str(&view.props, "time-zone") {
                        state.time_zone = tz.to_string();
                    }
                    if let Some(hf) = get_str(&view.props, "hour-format") {
                        state.hour_format = hf.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaFormatDate, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                "WaFormatNumber" => {
                    use rgui_components::wa_format_number::{WaFormatNumber, WaFormatNumberState};
                    let mut state = WaFormatNumberState::new();
                    if let Some(PropValue::Float(v)) = view.props.get("value") {
                        state.value = v.0;
                    }
                    if let Some(s) = get_str(&view.props, "style") {
                        state.style = s.to_string();
                    }
                    if let Some(PropValue::Bool(wg)) = view.props.get("without_grouping") {
                        state.without_grouping = *wg;
                    }
                    if let Some(c) = get_str(&view.props, "currency") {
                        state.currency = c.to_string();
                    }
                    if let Some(cd) = get_str(&view.props, "currency_display") {
                        state.currency_display = cd.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaFormatNumber, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                "WaRelativeTime" => {
                    use rgui_components::wa_relative_time::{WaRelativeTime, WaRelativeTimeState};
                    let mut state = WaRelativeTimeState::new();
                    if let Some(d) = get_str(&view.props, "date") {
                        state.date = d.to_string();
                    }
                    if let Some(f) = get_str(&view.props, "format") {
                        state.format = f.to_string();
                    }
                    if let Some(n) = get_str(&view.props, "numeric") {
                        state.numeric = n.to_string();
                    }
                    if let Some(PropValue::Bool(s)) = view.props.get("sync") {
                        state.sync = *s;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaRelativeTime, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                "WaAnimatedImage" => {
                    use rgui_components::wa_animated_image::{
                        WaAnimatedImage, WaAnimatedImageState,
                    };
                    let mut state = WaAnimatedImageState::new();
                    if let Some(v) = get_str(&view.props, "src") {
                        state.src = v.to_string();
                    }
                    if let Some(v) = get_str(&view.props, "alt") {
                        state.alt = v.to_string();
                    }
                    if let Some(PropValue::Bool(p)) = view.props.get("play") {
                        state.play = *p;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(
                        &WaAnimatedImage,
                        &state,
                        bounds,
                        &mut ctx,
                    );
                    ctx.into_operations()
                },

                "WaQrCode" => {
                    use rgui_components::wa_qr_code::{WaQrCode, WaQrCodeState};
                    let mut state = WaQrCodeState::new();
                    if let Some(v) = get_str(&view.props, "value") {
                        state.value = v.to_string();
                    }
                    if let Some(l) = get_str(&view.props, "label") {
                        state.label = l.to_string();
                    }
                    if let Some(PropValue::Float(s)) = view.props.get("size") {
                        state.size = s.0;
                    }
                    if let Some(PropValue::Float(r)) = view.props.get("radius") {
                        state.radius = r.0;
                    }
                    if let Some(ec) = get_str(&view.props, "error-correction") {
                        state.error_correction = ec.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaQrCode, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                "WaCarousel" => {
                    use rgui_components::wa_carousel::{WaCarousel, WaCarouselState};
                    let mut state = WaCarouselState::new();
                    if let Some(o) = get_str(&view.props, "orientation") {
                        state.orientation = o.to_string();
                    }
                    if let Some(PropValue::Bool(n)) = view.props.get("navigation") {
                        state.navigation = *n;
                    }
                    if let Some(PropValue::Bool(p)) = view.props.get("pagination") {
                        state.pagination = *p;
                    }
                    if let Some(PropValue::Int(spp)) = view.props.get("slides-per-page") {
                        state.slides_per_page = *spp as u32;
                    }
                    if let Some(PropValue::Int(spm)) = view.props.get("slides-per-move") {
                        state.slides_per_move = *spm as u32;
                    }
                    if let Some(PropValue::Int(s)) = view.props.get("slides") {
                        state.slides = *s as u32;
                    }
                    if let Some(PropValue::Int(cs)) = view.props.get("current-slide") {
                        state.current_slide = *cs as u32;
                    }
                    if let Some(PropValue::Int(a)) = view.props.get("active-slide") {
                        state.active_slide = *a as u32;
                    }
                    if let Some(PropValue::Bool(l)) = view.props.get("loop") {
                        state.r#loop = *l;
                    }
                    if let Some(PropValue::Bool(a)) = view.props.get("autoplay") {
                        state.autoplay = *a;
                    }
                    if let Some(PropValue::Bool(md)) = view.props.get("mouse-dragging") {
                        state.mouse_dragging = *md;
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaCarousel, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                "WaTreeItem" => {
                    use rgui_components::wa_tree_item::{WaTreeItem, WaTreeItemState};
                    let mut state = WaTreeItemState::new();
                    if let Some(l) = get_str(&view.props, "label") {
                        state.label = l.to_string();
                    }
                    if let Some(expanded) = view.props.get("expanded") {
                        if let PropValue::Bool(b) = expanded {
                            state.expanded = *b;
                        }
                    }
                    if let Some(selected) = view.props.get("selected") {
                        if let PropValue::Bool(b) = selected {
                            state.selected = *b;
                        }
                    }
                    if let Some(disabled) = view.props.get("disabled") {
                        if let PropValue::Bool(b) = disabled {
                            state.disabled = *b;
                        }
                    }
                    if let Some(lazy) = view.props.get("lazy") {
                        if let PropValue::Bool(b) = lazy {
                            state.lazy = *b;
                        }
                    }
                    if let Some(indeterminate) = view.props.get("indeterminate") {
                        if let PropValue::Bool(b) = indeterminate {
                            state.indeterminate = *b;
                        }
                    }
                    // 兼容 html! 宏不支持连字符属性名，同时读 is-leaf 和 is_leaf
                    if let Some(is_leaf) = view.props.get("is-leaf") {
                        if let PropValue::Bool(b) = is_leaf {
                            state.is_leaf = *b;
                        }
                    }
                    if let Some(is_leaf) = view.props.get("is_leaf") {
                        if let PropValue::Bool(b) = is_leaf {
                            state.is_leaf = *b;
                        }
                    }
                    if let Some(loading) = view.props.get("loading") {
                        if let PropValue::Bool(b) = loading {
                            state.loading = *b;
                        }
                    }
                    if let Some(selectable) = view.props.get("selectable") {
                        if let PropValue::Bool(b) = selectable {
                            state.selectable = *b;
                        }
                    }
                    if let Some(depth) = view.props.get("depth") {
                        if let PropValue::Int(d) = depth {
                            state.depth = *d as u32;
                        }
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaTreeItem, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },

                // ── 布局容器（自身不绘制）──
                "Container" | "Row" | "Column" | "Padding" | "Center" | "Expanded" | "SizedBox"
                | "Card" | "Stack" | "ScrollView" | "ListView" | "WaAnimation"
                | "WaButtonGroup" | "WaAccordion" | "WaCarouselItem" | "WaTree" => Vec::new(),

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

/// 创建默认的 PaintFn（无持久状态存储）。
///
/// 调度到已翻译组件的 paint() 方法。所有组件从 WidgetView.props 创建临时状态。
/// 需要持久交互状态时请使用 [`default_paint_fn_with_state`]。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
    paint_fn_impl(None)
}

/// 创建带实例状态存储的 PaintFn。
///
/// 与 [`default_paint_fn`] 相同，但对于交互式组件（如 WaAccordionItem），
/// 优先从 `store` 读取持久状态，而非每帧从 WidgetView.props 创建临时状态。
/// 这使得组件能够跨帧自主管理交互状态（如展开/折叠）。
#[must_use]
pub fn default_paint_fn_with_state<M: AppMessage>(store: WidgetStateStore) -> PaintFn<M> {
    paint_fn_impl(Some(store))
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
