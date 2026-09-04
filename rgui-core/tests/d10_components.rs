//! D10 组件行为测试（TDD RED→GREEN）：Accordion 折叠/展开交互 + WaBadge label。

use rgui_core::components::{
    Accordion, AccordionMsg, AccordionState, WaBadge, WaBadgeMsg, WaBadgeState,
};
use rgui_core::context::{UpdateContext, ViewContext};
use rgui_core::traits::WidgetSpec;
use rgui_core::view::PropValue;

fn contains_str(view: &rgui_core::view::WidgetView<AccordionMsg>, needle: &str) -> bool {
    let own_match = matches!(&view.props, PropValue::Str(s) if s.contains(needle));
    own_match
        || view
            .children
            .iter()
            .any(|c| contains_str(c, needle))
}

fn contains_str_badge(view: &rgui_core::view::WidgetView<WaBadgeMsg>, needle: &str) -> bool {
    let own_match = matches!(&view.props, PropValue::Str(s) if s.contains(needle));
    own_match
        || view
            .children
            .iter()
            .any(|c| contains_str_badge(c, needle))
}

#[test]
fn accordion_initially_collapsed() {
    let state = Accordion::initial_state();
    assert!(!state.expanded, "Accordion 初始应为收起状态");
    assert_eq!(state.title, "Accordion");
}

#[test]
fn accordion_toggle_expands_then_collapses() {
    let accordion = Accordion;
    let mut state = Accordion::initial_state();
    let ctx = &mut UpdateContext::default();

    accordion.update(AccordionMsg::Toggle, &mut state, ctx);
    assert!(state.expanded, "Toggle 后应展开");

    accordion.update(AccordionMsg::Toggle, &mut state, ctx);
    assert!(!state.expanded, "再次 Toggle 应收起");
}

#[test]
fn accordion_view_shows_content_when_expanded() {
    let accordion = Accordion;
    let ctx = &ViewContext::default();

    // 收起：内容不显示
    let mut collapsed = Accordion::initial_state();
    collapsed.expanded = false;
    let v_collapsed = accordion.view(&collapsed, ctx);
    assert!(
        !contains_str(&v_collapsed, "details"),
        "收起时不应显示内容"
    );

    // 展开：内容显示
    let mut expanded = Accordion::initial_state();
    expanded.expanded = true;
    let v_expanded = accordion.view(&expanded, ctx);
    assert!(
        contains_str(&v_expanded, "details"),
        "展开时应显示内容"
    );
}

#[test]
fn badge_view_shows_label_count() {
    let badge = WaBadge;
    let mut state = WaBadge::initial_state();
    state.count = 7;
    let v = badge.view(&state, &ViewContext::default());
    assert!(
        contains_str_badge(&v, "7"),
        "WaBadge 视图应包含数值 label，got props={:?}",
        v.props
    );
}

#[test]
fn badge_click_increments_count() {
    let badge = WaBadge;
    let mut state = WaBadge::initial_state();
    assert_eq!(state.count, 0);
    badge.update(WaBadgeMsg::Click, &mut state, &mut UpdateContext::default());
    assert_eq!(state.count, 1, "点击 WaBadge 应 count+1");
    badge.update(WaBadgeMsg::Click, &mut state, &mut UpdateContext::default());
    assert_eq!(state.count, 2, "再次点击应 count+1");
}

#[test]
fn accordion_and_badge_are_focusable() {
    assert!(Accordion.focusable(), "Accordion 应可获焦（Tab 导航）");
    assert!(WaBadge.focusable(), "WaBadge 应可获焦（Tab 导航）");
}

#[test]
fn accordion_view_adds_focus_marker_when_focused() {
    let state = Accordion::initial_state();
    // 未获焦：无焦点标记
    let ctx = &ViewContext::default();
    let v_norm = Accordion.view(&state, ctx);
    assert!(!contains_str(&v_norm, "▶"), "未获焦不应有焦点标记");
    // 获焦：标题带焦点前缀
    let mut ctx_f = ViewContext::default();
    ctx_f.focused = true;
    let v_foc = Accordion.view(&state, &ctx_f);
    assert!(contains_str(&v_foc, "▶"), "获焦应显示焦点高亮标记");
}

#[test]
fn badge_view_adds_focus_marker_when_focused() {
    let state = WaBadge::initial_state();
    let ctx = &ViewContext::default();
    let v_norm = WaBadge.view(&state, ctx);
    assert!(!contains_str_badge(&v_norm, "▶"), "未获焦不应有焦点标记");
    let mut ctx_f = ViewContext::default();
    ctx_f.focused = true;
    let v_foc = WaBadge.view(&state, &ctx_f);
    assert!(contains_str_badge(&v_foc, "▶"), "获焦应显示焦点高亮标记");
}
