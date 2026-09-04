//! D6 验收测试 · 真实 from_view 转换（WidgetView→SceneGraph：完整 Props + 布局 bounds）
//!
//! 注入：cp tools/qa/d6_tests/d6_acceptance_from_view.rs rgui-render/tests/
//! 运行：cargo test -p rgui-render --test d6_acceptance_from_view （无 GPU，A 层纯逻辑）
//!
//! 契约基线（dev 当前 scene_graph.rs）：
//!   `SceneGraph::from_view<M>(&WidgetView<M>) -> Self`（单参；容器 = 根 size 或 DEFAULT_CONTAINER）
//!   - Color → FillRect（用 view.size / slot.size）
//!   - Str  → DrawText（默认灰 200，文本色非背景）
//!   - Unit/Bool/Int/Float/WidgetId → 无图元
//!   布局：LayoutEngine::compute_children(slot.size, &child_sizes) 求子节点真实 bounds。
//! 与 dev tests/e2e_from_view.rs 互补：dev 已验 Color/Str/端到端，本骨架补 Int/Float/Unit/递归坐标/文本色/bounds。

use rgui_core::geometry::Size;
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_render::scene_graph::{DrawCmd, SceneGraph};

type M = ();

fn view_with(props: PropValue, children: Vec<WidgetView<M>>) -> WidgetView<M> {
    let mut v = WidgetView::empty();
    v.props = props;
    v.children = children;
    v
}

fn color_view(c: Color) -> WidgetView<M> {
    view_with(PropValue::Color(c), vec![])
}

fn count_cmds(sg: &SceneGraph) -> usize { sg.cmds().len() }

// ============ Color → FillRect ============

#[test]
fn fv1_color_maps_to_fillrect() {
    let sg = SceneGraph::from_view(&color_view(Color::rgb(255, 0, 0)));
    let cmds = sg.cmds();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        DrawCmd::FillRect { color, .. } => assert_eq!(*color, Color::rgb(255, 0, 0)),
        _ => panic!("Color prop 应映射为 FillRect"),
    }
}

// ============ Str → DrawText ============

#[test]
fn fv2_str_maps_to_drawtext() {
    let sg = SceneGraph::from_view(&view_with(PropValue::Str("Hello".into()), vec![]));
    let cmds = sg.cmds();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        DrawCmd::DrawText { text, size, color, .. } => {
            assert_eq!(text, "Hello");
            assert!(*size > 0.0, "DrawText size 应 > 0");
            assert!(color.r > 100 || color.g > 100 || color.b > 100, "文本色应非背景可见(当前灰200)");
        }
        _ => panic!("Str prop 应映射为 DrawText"),
    }
}

// ============ Unit/Bool/Int/Float → 无图元（容器） ============

#[test]
fn fv3_unit_node_no_graphics() {
    let sg = SceneGraph::from_view(&WidgetView::<M>::empty());
    assert_eq!(count_cmds(&sg), 0, "Unit 容器节点不应产生图元");
}

#[test]
fn fv4_int_float_bool_no_graphics() {
    for props in [PropValue::Int(7), PropValue::Float(3.14), PropValue::Bool(true)] {
        let sg = SceneGraph::from_view(&view_with(props.clone(), vec![]));
        assert_eq!(count_cmds(&sg), 0, "{props:?} 不应产生图元（无静默错误）");
    }
}

// ============ Size 字段影响 bounds（布局真实作用） ============

#[test]
fn fv5_size_field_affects_fillrect_bounds() {
    let mut v = color_view(Color::rgb(0, 0, 255));
    v.size = Some(Size::new(120.0, 60.0));
    let sg = SceneGraph::from_view(&v);
    match &sg.cmds()[0] {
        DrawCmd::FillRect { width, height, .. } => {
            assert!((*width - 120.0).abs() < 0.001, "width 应=120, got {width}");
            assert!((*height - 60.0).abs() < 0.001, "height 应=60, got {height}");
        }
        _ => panic!("应为 FillRect"),
    }
}

// ============ 递归子节点：布局坐标顺序 + 数量 ============

#[test]
fn fv6_recursive_children_ordered() {
    let parent = view_with(
        PropValue::Unit,
        vec![
            color_view(Color::rgb(255, 0, 0)),
            color_view(Color::rgb(0, 255, 0)),
            view_with(PropValue::Str("Hi".into()), vec![]),
        ],
    );
    let sg = SceneGraph::from_view(&parent);
    let cmds = sg.cmds();
    assert_eq!(cmds.len(), 3, "应产生 3 条图元（红/绿/文本）");
    assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == Color::rgb(255,0,0)));
    assert!(matches!(&cmds[1], DrawCmd::FillRect { color, .. } if *color == Color::rgb(0,255,0)));
    assert!(matches!(&cmds[2], DrawCmd::DrawText { text, .. } if text == "Hi"));
}

#[test]
fn fv7_child_positions_use_layout_not_fixed() {
    let parent = view_with(
        PropValue::Unit,
        vec![
            color_view(Color::rgb(255, 0, 0)),
            color_view(Color::rgb(0, 255, 0)),
        ],
    );
    let sg = SceneGraph::from_view(&parent);
    let cmds = sg.cmds();
    let y0 = match &cmds[0] { DrawCmd::FillRect { y, .. } => *y, _ => panic!() };
    let y1 = match &cmds[1] { DrawCmd::FillRect { y, .. } => *y, _ => panic!() };
    assert!(y1 >= y0, "布局有序：子节点 y 应递增（y1={y1} >= y0={y0}）");
}

// ============ 递归坐标不越界（布局真实作用） ============

#[test]
fn fv8_child_bounds_within_container() {
    let mut root = color_view(Color::rgb(255, 0, 0));
    root.size = Some(Size::new(200.0, 200.0));
    root.children.push(color_view(Color::rgb(0, 255, 0)));
    let sg = SceneGraph::from_view(&root);
    let cmds = sg.cmds();
    assert_eq!(cmds.len(), 2);
    // 子节点坐标应落在容器内（非负数、不超容器尺寸）
    if let DrawCmd::FillRect { x, y, width, height, .. } = &cmds[1] {
        assert!(*x >= 0.0 && *y >= 0.0, "子节点坐标非负: x={x} y={y}");
        assert!(*x + *width <= 200.0 + 0.001, "子不超容器宽: x+width={}", *x + *width);
        assert!(*y + *height <= 200.0 + 0.001, "子不超容器高: y+height={}", *y + *height);
    }
}
