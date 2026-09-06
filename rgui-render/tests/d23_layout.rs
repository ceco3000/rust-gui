//! D23 残留项自证：P1-1 details 独立成区 / P1-2 内容 20pt 边距 / P0-2 文字垂直居中（bbox）。

#![cfg(feature = "vello-backend")]

use rgui_core::geometry::Size;
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_render::scene_graph::{DrawCmd, SceneGraph};

/// 提取某颜色的 FillRect 坐标（按出现顺序）。
fn fill_rects_with_color(cmds: &[DrawCmd], target: Color) -> Vec<(f32, f32)> {
    cmds.iter()
        .filter_map(|c| match c {
            DrawCmd::FillRect { x, y, color, .. } if *color == target => Some((*x, *y)),
            _ => None,
        })
        .collect()
}

/// 提取 DrawText 的 (y, text)。
fn draw_texts(cmds: &[DrawCmd]) -> Vec<(f32, String)> {
    cmds.iter()
        .filter_map(|c| match c {
            DrawCmd::DrawText { y, text, .. } => Some((*y, text.clone())),
            _ => None,
        })
        .collect()
}

/// P1-1：Accordion 展开时，内容（content）应位于 header 之下（纵向独立区），非与 header 并排。
#[test]
fn accordion_content_below_header_when_expanded() {
    use rgui_core::components::{Accordion, AccordionState};
    use rgui_core::context::ViewContext;
    use rgui_core::traits::WidgetSpec;
    let mut state = AccordionState::default();
    state.expanded = true;
    let v = Accordion.view(&state, &ViewContext::default());
    let scene = SceneGraph::from_view(&v);
    let header_y = scene
        .cmds()
        .iter()
        .find_map(|c| match c {
            DrawCmd::FillRect { y, .. } => Some(*y),
            _ => None,
        })
        .expect("应有 header FillRect");
    // content 是展开态 content 子视图的 DrawText（其文本为 subtitle）
    let content_y = draw_texts(scene.cmds())
        .into_iter()
        .filter(|(y, t)| *y > header_y + 10.0 && !t.contains("▸") && !t.contains("▾"))
        .map(|(y, _)| y)
        .next()
        .expect("展开态应有 content DrawText 在 header 之下");
    assert!(
        content_y > header_y + 10.0,
        "content 应在 header 下方（独立纵向区），header_y={header_y} content_y={content_y}"
    );
}

/// P1-2：容器 padding=20 → 内容不贴左缘（child FillRect x=20）。
#[test]
fn container_padding_insets_children_20pt() {
    let mut root: WidgetView<()> = WidgetView::empty();
    root.size = Some(Size::new(520.0, 220.0));
    root.padding = 20.0;
    let mut child: WidgetView<()> = WidgetView::empty();
    child.props = PropValue::Color(Color::rgb(0, 122, 255));
    child.size = Some(Size::new(340.0, 44.0));
    root.children.push(child);
    let scene = SceneGraph::from_view(&root);
    let fills = fill_rects_with_color(scene.cmds(), Color::rgb(0, 122, 255));
    assert_eq!(fills.len(), 1, "应恰好一个内容 FillRect");
    assert!(
        (fills[0].0 - 20.0).abs() < 0.01,
        "内容应内缩 20pt（不再贴左缘），got x={}",
        fills[0].0
    );
    assert!(
        (fills[0].1 - 20.0).abs() < 0.01,
        "内容顶部也应内缩 20pt，got y={}",
        fills[0].1
    );
}

/// P0-2：header 文字像素 bbox 中心 应接近 header 矩形中心（≤2pt）。
#[test]
fn accordion_header_text_vcentered() {
    use rgui_core::components::{Accordion, AccordionState};
    use rgui_core::context::ViewContext;
    use rgui_core::traits::WidgetSpec;
    use rgui_render::vello::VelloBackend;

    let mut backend = VelloBackend::new().expect("backend");
    let v = Accordion.view(&AccordionState::default(), &ViewContext::default());
    let scene = SceneGraph::from_view(&v);
    let w = 340u32;
    let h = 44u32; // 收起 Accordion 高
    let pixels = backend.render_offscreen(&scene, w, h).expect("renders");

    // 文字前景 #E8E8E8 → linear 读回 ≈ (229,229,229)；is_text：亮灰（r/g/b 都高且相近）
    let is_text = |i: usize| pixels[i] > 180 && pixels[i + 1] > 180 && pixels[i + 2] > 180;
    // 计算文字像素的 y 范围（bbox）
    let mut ys: Vec<u32> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            if is_text(i) {
                ys.push(y);
            }
        }
    }
    assert!(!ys.is_empty(), "应检测到 header 文字像素");
    let text_center_y = (ys.iter().min().unwrap() + ys.iter().max().unwrap()) as f32 / 2.0;
    // 文字应垂直居中于 header（36pt 标题行）中心 = 18（Accordion root 收起 44 高，header 顶部 36）
    let header_center = 18.0;
    let diff = (text_center_y - header_center).abs();
    assert!(
        diff <= 2.0,
        "文字 bbox 中心应≈header 中心（≤2pt），text_center_y={text_center_y} header_center={header_center} diff={diff}"
    );
}
