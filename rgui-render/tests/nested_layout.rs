//! P2-1 嵌套布局坐标累加测试（TDD RED 起点）。
//!
//! 目标：`emit_node` 递归时应把**父节点偏移累加**到子/孙节点坐标。
//! 嵌套结构（父含子含孙）时，孙节点绝对坐标 = 父偏移 + 子偏移 + 孙相对。
//! 当前实现未累加（只传相对 slot）→ 嵌套组件错位（RED）。

use rgui_core::geometry::Size;
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_render::scene_graph::{DrawCmd, SceneGraph};

type M = ();

fn node_with(props: PropValue, size: Size, children: Vec<WidgetView<M>>) -> WidgetView<M> {
    let mut v = WidgetView::empty();
    v.props = props;
    v.size = Some(size);
    v.children = children;
    v
}

/// 提取某颜色的 FillRect 坐标列表（按出现顺序）。
fn fill_rect_xy(cmds: &[DrawCmd], target: Color) -> Vec<(f32, f32)> {
    cmds.iter()
        .filter_map(|c| match c {
            DrawCmd::FillRect { x, y, color, .. } if *color == target => Some((*x, *y)),
            _ => None,
        })
        .collect()
}

#[test]
fn nested_widget_position_accumulates_parent_offset() {
    // 结构：根(白，200x200) → [占位子(白，100x80), 目标子(绿，100x40 → 内含孙(红，100x20))]
    // taffy 默认 flex 方向为 Row（横向）：占位子在 x=0(宽80)，目标子在其后 x=80，
    // 孙在目标子容器内相对 x=0。累加后：孙绝对 x = 80(目标子在根内) + 0 = 80。
    let target_child = node_with(
        PropValue::Color(Color::rgb(0, 255, 0)),
        Size::new(100.0, 40.0),
        vec![node_with(
            PropValue::Color(Color::rgb(255, 0, 0)),
            Size::new(100.0, 20.0),
            vec![],
        )],
    );
    let root = node_with(
        PropValue::Unit,
        Size::new(200.0, 200.0),
        vec![
            node_with(
                PropValue::Color(Color::rgb(255, 255, 255)),
                Size::new(120.0, 80.0),
                vec![],
            ),
            target_child,
        ],
    );

    let graph = SceneGraph::from_view(&root);
    let cmds = graph.cmds();

    // 目标子（绿）应布局在占位子之后（row 布局）：x ≈ 120
    let green = fill_rect_xy(cmds, Color::rgb(0, 255, 0));
    assert_eq!(green.len(), 1, "应恰好一个绿色 FillRect");
    assert!(
        green[0].0 >= 100.0,
        "目标子应位于占位子之后（x>=100，因累加父容器内偏移），got x={}",
        green[0].0
    );

    // 孙节点（红）绝对 x = 目标子 x + 孙内部相对 x（>= 目标子 x）——体现**父偏移累加**
    let red = fill_rect_xy(cmds, Color::rgb(255, 0, 0));
    assert_eq!(red.len(), 1, "应恰好一个红色孙 FillRect");
    assert!(
        red[0].0 >= green[0].0,
        "孙节点绝对 x 应累加父偏移（>= 目标子 x），got 孙x={} vs 子x={}",
        red[0].0,
        green[0].0
    );
}
