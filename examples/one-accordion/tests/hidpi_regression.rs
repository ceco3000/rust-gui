use rgui::AppMessage;
use rgui::app::{AppConfig, InteractionAutomationHarness};
use rgui::geometry::{Point, Rect};
use rgui::wa_accordion_item::WaAccordionItemState;
use rgui::{Event, MouseEventCoords, MouseInputOrigin};

#[derive(Debug, Clone, PartialEq, AppMessage)]
#[allow(dead_code)]
enum Msg {
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InteractionSnapshot {
    initial: (bool, bool),
    hover_section_2: Option<String>,
    click_section_2: Option<String>,
    after_click_section_2: (bool, bool),
    hover_section_1: Option<String>,
    click_section_1: Option<String>,
    after_click_section_1: (bool, bool),
}

fn make_probe() -> InteractionAutomationHarness<Msg> {
    InteractionAutomationHarness::from_config(
        AppConfig::new()
            .window_size(350.0, 250.0)
            .rgui_path(concat!(env!("CARGO_MANIFEST_DIR"), "/ui.rgui"))
            .rhai_paths(vec![concat!(env!("CARGO_MANIFEST_DIR"), "/handlers.rhai").into()]),
    )
    .expect("创建交互自动化 harness 失败")
}

fn section_ids(probe: &InteractionAutomationHarness<Msg>) -> (rgui::WidgetId, rgui::WidgetId) {
    (
        probe.widget_id("section-1").expect("section-1 未分配 WidgetId"),
        probe.widget_id("section-2").expect("section-2 未分配 WidgetId"),
    )
}

fn header_center(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + 22.0)
}

fn raw_platform_header_center(
    probe: &InteractionAutomationHarness<Msg>,
    widget_id: rgui::WidgetId,
) -> Point {
    let logical = header_center(probe.widget_rect(widget_id).expect("widget 缺少布局 rect"));
    #[cfg(target_os = "macos")]
    {
        logical
    }
    #[cfg(not(target_os = "macos"))]
    {
        logical.scale(probe.scale_factor())
    }
}

fn header_local_point(probe: &InteractionAutomationHarness<Msg>, widget_id: rgui::WidgetId) -> Point {
    let rect = probe.widget_rect(widget_id).expect("widget 缺少布局 rect");
    let logical = header_center(rect);
    Point::new(logical.x - rect.origin.x, logical.y - rect.origin.y)
}

fn expanded_flags(
    probe: &InteractionAutomationHarness<Msg>,
    section_1: rgui::WidgetId,
    section_2: rgui::WidgetId,
) -> (bool, bool) {
    let state_1 = probe
        .widget_state::<WaAccordionItemState>(section_1)
        .expect("section-1 缺少状态");
    let state_2 = probe
        .widget_state::<WaAccordionItemState>(section_2)
        .expect("section-2 缺少状态");
    (state_1.expanded, state_2.expanded)
}

fn widget_name(
    probe: &InteractionAutomationHarness<Msg>,
    widget_id: Option<rgui::WidgetId>,
) -> Option<String> {
    widget_id.and_then(|id| probe.widget_name(id).map(str::to_string))
}

fn last_mouse_move_coords(events: &[Event]) -> MouseEventCoords {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::MouseMove { coords, .. } => Some(*coords),
            _ => None,
        })
        .expect("缺少 MouseMove 事件")
}

fn last_mouse_down_coords(events: &[Event]) -> MouseEventCoords {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::MouseDown { coords, .. } => Some(*coords),
            _ => None,
        })
        .expect("缺少 MouseDown 事件")
}

fn last_drag_enter_coords(events: &[Event]) -> MouseEventCoords {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::DragEnter { coords } => Some(*coords),
            _ => None,
        })
        .expect("缺少 DragEnter 事件")
}

fn last_drag_over_coords(events: &[Event]) -> MouseEventCoords {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::DragOver { coords } => Some(*coords),
            _ => None,
        })
        .expect("缺少 DragOver 事件")
}

fn last_drop_coords(events: &[Event]) -> MouseEventCoords {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::Drop { coords } => Some(*coords),
            _ => None,
        })
        .expect("缺少 Drop 事件")
}

fn assert_platform_origin(
    coords: MouseEventCoords,
    expected_window_logical: Point,
    expected_local_logical: Point,
    expected_raw_window_position: Point,
) {
    assert_eq!(coords.window_logical, expected_window_logical);
    assert_eq!(coords.local_logical, Some(expected_local_logical));
    assert!(matches!(
        coords.origin,
        MouseInputOrigin::PlatformWindowEvent {
            raw_window_position,
            ..
        } if raw_window_position == expected_raw_window_position
    ));
}

fn run_scenario(scale_factor: f64) -> InteractionSnapshot {
    let mut probe = make_probe();
    probe.set_scale_factor(scale_factor);
    let (section_1, section_2) = section_ids(&probe);

    let initial = expanded_flags(&probe, section_1, section_2);

    let section_2_point = raw_platform_header_center(&probe, section_2);
    let hovered_section_2 = probe.inject_hover_platform_window_raw(section_2_point);
    let hover_section_2 = widget_name(&probe, hovered_section_2);
    let clicked_section_2 = probe.inject_click_platform_window_raw(section_2_point);
    let click_section_2 = widget_name(&probe, clicked_section_2);
    let after_click_section_2 = expanded_flags(&probe, section_1, section_2);

    let section_1_point = raw_platform_header_center(&probe, section_1);
    let hovered_section_1 = probe.inject_hover_platform_window_raw(section_1_point);
    let hover_section_1 = widget_name(&probe, hovered_section_1);
    let clicked_section_1 = probe.inject_click_platform_window_raw(section_1_point);
    let click_section_1 = widget_name(&probe, clicked_section_1);
    let after_click_section_1 = expanded_flags(&probe, section_1, section_2);

    InteractionSnapshot {
        initial,
        hover_section_2,
        click_section_2,
        after_click_section_2,
        hover_section_1,
        click_section_1,
        after_click_section_1,
    }
}

#[test]
fn retina_platform_raw_injection_hits_visual_target() {
    let mut probe = make_probe();
    probe.set_scale_factor(2.0);
    let (section_1, section_2) = section_ids(&probe);

    let section_2_logical = header_center(probe.widget_rect(section_2).expect("section-2 rect 缺失"));
    let section_2_platform_raw = raw_platform_header_center(&probe, section_2);
    assert_eq!(probe.hit_test_logical(section_2_logical), Some(section_2));
    assert_eq!(
        probe.inject_hover_platform_window_raw(section_2_platform_raw),
        Some(section_2)
    );
    assert_eq!(
        probe.inject_click_platform_window_raw(section_2_platform_raw),
        Some(section_2)
    );
    assert_eq!(expanded_flags(&probe, section_1, section_2), (false, true));

    let section_1_logical = header_center(probe.widget_rect(section_1).expect("section-1 rect 缺失"));
    let section_1_platform_raw = raw_platform_header_center(&probe, section_1);
    assert_eq!(probe.hit_test_logical(section_1_logical), Some(section_1));
    assert_eq!(
        probe.inject_hover_platform_window_raw(section_1_platform_raw),
        Some(section_1)
    );
    assert_eq!(
        probe.inject_click_platform_window_raw(section_1_platform_raw),
        Some(section_1)
    );
    assert_eq!(expanded_flags(&probe, section_1, section_2), (true, false));
}

#[test]
fn logical_raw_injection_and_window_replay_keep_coordinate_semantics_aligned() {
    let logical_probe = {
        let mut probe = make_probe();
        probe.set_scale_factor(2.0);
        probe
    };
    let (section_1, section_2) = section_ids(&logical_probe);
    let section_2_logical = header_center(
        logical_probe
            .widget_rect(section_2)
            .expect("section-2 rect 缺失"),
    );
    let section_2_local = header_local_point(&logical_probe, section_2);
    let section_2_platform_raw = raw_platform_header_center(&logical_probe, section_2);

    let mut logical_probe = logical_probe;
    logical_probe.clear_events();
    assert_eq!(
        logical_probe.inject_click_logical(section_2_logical),
        Some(section_2)
    );
    let logical_coords = last_mouse_down_coords(logical_probe.events());
    assert_eq!(logical_coords.window_logical, section_2_logical);
    assert_eq!(logical_coords.local_logical, Some(section_2_local));
    assert!(matches!(
        logical_coords.origin,
        MouseInputOrigin::LogicalInjection
    ));
    assert_eq!(expanded_flags(&logical_probe, section_1, section_2), (false, true));

    let mut raw_probe = make_probe();
    raw_probe.set_scale_factor(2.0);
    let (raw_section_1, raw_section_2) = section_ids(&raw_probe);
    raw_probe.clear_events();
    assert_eq!(
        raw_probe.inject_click_platform_window_raw(section_2_platform_raw),
        Some(raw_section_2)
    );
    let raw_coords = last_mouse_down_coords(raw_probe.events());
    assert_eq!(raw_coords.window_logical, section_2_logical);
    assert_eq!(raw_coords.local_logical, Some(section_2_local));
    assert!(matches!(
        raw_coords.origin,
        MouseInputOrigin::PhysicalInjection {
            raw_window_position,
            ..
        } if raw_window_position == section_2_platform_raw
    ));
    assert_eq!(expanded_flags(&raw_probe, raw_section_1, raw_section_2), (false, true));

    let mut replay_probe = make_probe();
    replay_probe.set_scale_factor(2.0);
    let (replay_section_1, replay_section_2) = section_ids(&replay_probe);
    replay_probe.clear_events();
    assert_eq!(
        replay_probe.replay_left_click_platform_window_raw(section_2_platform_raw),
        Some(replay_section_2)
    );
    let replay_move_coords = last_mouse_move_coords(replay_probe.events());
    let replay_down_coords = last_mouse_down_coords(replay_probe.events());
    assert_eq!(replay_move_coords.window_logical, section_2_logical);
    assert_eq!(replay_move_coords.local_logical, Some(section_2_local));
    assert!(matches!(
        replay_move_coords.origin,
        MouseInputOrigin::PlatformWindowEvent {
            raw_window_position,
            ..
        } if raw_window_position == section_2_platform_raw
    ));
    assert_eq!(replay_down_coords.window_logical, section_2_logical);
    assert_eq!(replay_down_coords.local_logical, Some(section_2_local));
    assert!(matches!(
        replay_down_coords.origin,
        MouseInputOrigin::PlatformWindowEvent {
            raw_window_position,
            ..
        } if raw_window_position == section_2_platform_raw
    ));
    assert_eq!(
        expanded_flags(&replay_probe, replay_section_1, replay_section_2),
        (false, true)
    );
}

#[test]
fn drag_events_keep_logical_raw_and_window_replay_coordinate_semantics_aligned() {
    let logical_probe = {
        let mut probe = make_probe();
        probe.set_scale_factor(2.0);
        probe
    };
    let (_, section_2) = section_ids(&logical_probe);
    let section_2_logical = header_center(
        logical_probe
            .widget_rect(section_2)
            .expect("section-2 rect 缺失"),
    );
    let section_2_local = header_local_point(&logical_probe, section_2);
    let section_2_platform_raw = raw_platform_header_center(&logical_probe, section_2);

    let mut logical_probe = logical_probe;
    logical_probe.clear_events();
    assert_eq!(
        logical_probe.inject_drag_enter_logical(section_2_logical),
        Some(section_2)
    );
    assert_eq!(
        logical_probe.inject_drag_over_logical(section_2_logical),
        Some(section_2)
    );
    assert_eq!(
        logical_probe.inject_drop_logical(section_2_logical),
        Some(section_2)
    );
    for coords in [
        last_drag_enter_coords(logical_probe.events()),
        last_drag_over_coords(logical_probe.events()),
        last_drop_coords(logical_probe.events()),
    ] {
        assert_eq!(coords.window_logical, section_2_logical);
        assert_eq!(coords.local_logical, Some(section_2_local));
        assert!(matches!(coords.origin, MouseInputOrigin::LogicalInjection));
    }

    let mut raw_probe = make_probe();
    raw_probe.set_scale_factor(2.0);
    let (_, raw_section_2) = section_ids(&raw_probe);
    raw_probe.clear_events();
    assert_eq!(
        raw_probe.inject_drag_enter_platform_window_raw(section_2_platform_raw),
        Some(raw_section_2)
    );
    assert_eq!(
        raw_probe.inject_drag_over_platform_window_raw(section_2_platform_raw),
        Some(raw_section_2)
    );
    assert_eq!(
        raw_probe.inject_drop_platform_window_raw(section_2_platform_raw),
        Some(raw_section_2)
    );
    for coords in [
        last_drag_enter_coords(raw_probe.events()),
        last_drag_over_coords(raw_probe.events()),
        last_drop_coords(raw_probe.events()),
    ] {
        assert_eq!(coords.window_logical, section_2_logical);
        assert_eq!(coords.local_logical, Some(section_2_local));
        assert!(matches!(
            coords.origin,
            MouseInputOrigin::PhysicalInjection {
                raw_window_position,
                ..
            } if raw_window_position == section_2_platform_raw
        ));
    }

    let mut replay_probe = make_probe();
    replay_probe.set_scale_factor(2.0);
    let (_, replay_section_2) = section_ids(&replay_probe);
    replay_probe.clear_events();
    assert_eq!(
        replay_probe.replay_drag_enter_platform_window_raw(section_2_platform_raw),
        Some(replay_section_2)
    );
    assert_eq!(
        replay_probe.replay_drag_over_platform_window_raw(section_2_platform_raw),
        Some(replay_section_2)
    );
    assert_eq!(
        replay_probe.replay_drop_platform_window_raw(section_2_platform_raw),
        Some(replay_section_2)
    );
    for coords in [
        last_drag_enter_coords(replay_probe.events()),
        last_drag_over_coords(replay_probe.events()),
        last_drop_coords(replay_probe.events()),
    ] {
        assert_eq!(coords.window_logical, section_2_logical);
        assert_eq!(coords.local_logical, Some(section_2_local));
        assert!(matches!(
            coords.origin,
            MouseInputOrigin::PlatformWindowEvent {
                raw_window_position,
                ..
            } if raw_window_position == section_2_platform_raw
        ));
    }
}

#[test]
fn scale_factor_1_and_2_keep_hover_and_click_behavior_consistent() {
    let scale_1 = run_scenario(1.0);
    let scale_2 = run_scenario(2.0);

    assert_eq!(
        scale_1, scale_2,
        "scale_factor=1 与 scale_factor=2 的 hover/click 行为应保持一致"
    );
    assert_eq!(scale_1.initial, (true, false));
    assert_eq!(scale_1.hover_section_2.as_deref(), Some("section-2"));
    assert_eq!(scale_1.click_section_2.as_deref(), Some("section-2"));
    assert_eq!(scale_1.after_click_section_2, (false, true));
    assert_eq!(scale_1.hover_section_1.as_deref(), Some("section-1"));
    assert_eq!(scale_1.click_section_1.as_deref(), Some("section-1"));
    assert_eq!(scale_1.after_click_section_1, (true, false));
}

#[test]
fn cross_display_scale_switch_keeps_qt_coordinate_relationships_consistent() {
    let mut probe = make_probe();
    let (section_1, section_2) = section_ids(&probe);
    let section_2_logical = header_center(probe.widget_rect(section_2).expect("section-2 rect 缺失"));
    let section_2_local = header_local_point(&probe, section_2);

    probe.set_scale_factor(1.0);
    let raw_scale_1 = raw_platform_header_center(&probe, section_2);
    probe.clear_events();
    assert_eq!(
        probe.replay_cursor_moved_platform_window_raw(raw_scale_1),
        Some(section_2)
    );
    let scale_1_move_coords = last_mouse_move_coords(probe.events());
    assert_platform_origin(
        scale_1_move_coords,
        section_2_logical,
        section_2_local,
        raw_scale_1,
    );

    probe.set_scale_factor(2.0);
    let raw_scale_2 = raw_platform_header_center(&probe, section_2);
    #[cfg(target_os = "macos")]
    assert_eq!(raw_scale_2, raw_scale_1);
    #[cfg(not(target_os = "macos"))]
    assert_eq!(raw_scale_2, section_2_logical.scale(2.0));

    probe.clear_events();
    assert_eq!(
        probe.replay_left_click_platform_window_raw(raw_scale_2),
        Some(section_2)
    );
    let scale_2_move_coords = last_mouse_move_coords(probe.events());
    let scale_2_down_coords = last_mouse_down_coords(probe.events());
    assert_platform_origin(
        scale_2_move_coords,
        section_2_logical,
        section_2_local,
        raw_scale_2,
    );
    assert_platform_origin(
        scale_2_down_coords,
        section_2_logical,
        section_2_local,
        raw_scale_2,
    );
    assert_eq!(expanded_flags(&probe, section_1, section_2), (false, true));

    probe.set_scale_factor(1.0);
    let raw_scale_1_again = raw_platform_header_center(&probe, section_1);
    probe.clear_events();
    assert_eq!(
        probe.replay_left_click_platform_window_raw(raw_scale_1_again),
        Some(section_1)
    );
    let scale_1_again_down_coords = last_mouse_down_coords(probe.events());
    assert_platform_origin(
        scale_1_again_down_coords,
        header_center(probe.widget_rect(section_1).expect("section-1 rect 缺失")),
        header_local_point(&probe, section_1),
        raw_scale_1_again,
    );
    assert_eq!(expanded_flags(&probe, section_1, section_2), (true, false));
}
