//! D6 端到端测试（TDD RED 起点）：真实 `WidgetView` → `from_view` → 离屏渲染 → 像素断言。
//!
//! 替换 D5 手工 `red_filled_rect`——本测试从含 Color+布局+文本的 **真实 WidgetView**
//! 转换渲染，验证 from_view 转换正确性（布局应用 + 完整 Props 映射）。

#![cfg(feature = "vello-backend")]

use rgui_core::traits::AppMessage;
use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_render::scene_graph::{DrawCmd, SceneGraph};
use rgui_render::vello::VelloBackend;

#[derive(Debug, Clone)]
struct TestMsg;
impl AppMessage for TestMsg {
    fn message_name(&self) -> &'static str {
        "test"
    }
}

fn color_view() -> WidgetView<TestMsg> {
    let mut v = WidgetView::empty();
    v.props = PropValue::Color(Color::rgb(255, 0, 0));
    v
}

fn text_child() -> WidgetView<TestMsg> {
    let mut v = WidgetView::empty();
    v.props = PropValue::Str("Hello".to_string());
    v
}

#[test]
fn from_view_color_rect_becomes_fill_rect_cmd() {
    let view = color_view();
    let graph = SceneGraph::from_view(&view);

    let cmds = graph.cmds();
    assert!(
        cmds.iter().any(|c| matches!(c, DrawCmd::FillRect { color, .. } if color.r == 255 && color.g == 0 && color.b == 0)),
        "from_view 应把 Color props 转为红色 FillRect"
    );
}

#[test]
fn from_view_text_props_become_draw_text_cmd() {
    let mut view = color_view();
    view.children.push(text_child());
    let graph = SceneGraph::from_view(&view);

    let cmds = graph.cmds();
    assert!(
        cmds.iter().any(|c| matches!(c, DrawCmd::DrawText { text, .. } if text == "Hello")),
        "from_view 应把 Str props 转为 DrawText（而非静默忽略）"
    );
}

#[test]
fn e2e_widgetview_red_square_renders_to_pixels() {
    let view = color_view();
    let graph = SceneGraph::from_view(&view);

    let mut backend = VelloBackend::new().expect("wgpu vello backend");
    let pixels = backend
        .render_offscreen(&graph, 64, 64)
        .expect("offscreen render");

    assert_eq!(pixels.len(), 64 * 64 * 4);
    // 根矩形从 (0,0) 起 → 采样中心应为红色
    let idx = (32 * 64 + 32) * 4;
    let (r, g, b) = (pixels[idx], pixels[idx + 1], pixels[idx + 2]);
    assert!(r > 200, "R expected high, got {r}");
    assert!(g < 60, "G expected low, got {g}");
    assert!(b < 60, "B expected low, got {b}");
}
