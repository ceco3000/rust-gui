//! D6 验收测试 · 真实 WidgetView 端到端（→ SceneGraph → 离屏渲染 → 像素级断言）
//!
//! 注入：cp tools/qa/d6_tests/d6_acceptance_e2e.rs rgui-render/tests/
//! 运行：cargo test -p rgui-render --features vello-backend --test d6_acceptance_e2e
//!
//! ⚠️ 环境依赖：需要 wgpu/vello GPU 或软适配器（同 D5 offscreen）。不可用时报告"环境依赖"。
//! ⚠️ 契约锁定：本文件验证 D6 end-to-end（真实 WidgetView→from_view→render_offscreen→像素断言），
//!      替换 D5 的手工 red_filled_rect 测试。所有断言待 dev 实现真实 from_view 后 PASS。
//!
//! 参照：core::view::WidgetView/PropValue/Color；render::scene_graph::{SceneGraph,DrawCmd}；
//!      render::vello::{VelloBackend,RenderBackend}（render_offscreen -> Result<Vec<u8>,_>，RGBA 紧密）。

#![cfg(feature = "vello-backend")]

use rgui_core::view::{Color, PropValue, WidgetView};
use rgui_render::scene_graph::SceneGraph;
use rgui_render::vello::{RenderBackend, VelloBackend};

type M = ();

fn view_with(props: PropValue, children: Vec<WidgetView<M>>) -> WidgetView<M> {
    let mut v = WidgetView::empty();
    v.props = props;
    v.children = children;
    v
}

/// 读回 (x,y) 像素的 RGBA（y*width+x 索引，x/y 为 f32 取整）。
fn pixel_at(pixels: &[u8], width: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let idx = ((y * width + x) * 4) as usize;
    (pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3])
}

// ============ E2E: 像素级（dev 实现真实 from_view 后 PASS） ============

#[test]
fn e2e1_real_color_view_renders_pixels() {
    let mut backend = VelloBackend::new().expect("vello backend");
    let view = view_with(PropValue::Color(Color::rgb(0, 0, 255)), vec![]);
    let scene = SceneGraph::from_view(&view);
    let (w, h) = (64u32, 64u32);
    let pixels = backend.render_offscreen(&scene, w, h).expect("offscreen");
    assert_eq!(pixels.len(), (w * h * 4) as usize);
    // 中心像素应是蓝色（替换 D5 red_filled_rect）
    let (r, g, b, _a) = pixel_at(&pixels, w, 32, 32);
    assert!(b > 200, "B channel high, got {b}");
    assert!(r < 60, "R channel low, got {r}");
    assert!(g < 60, "G channel low, got {g}");
}

#[test]
fn e2e2_text_view_produces_pixels() {
    let mut backend = VelloBackend::new().expect("vello backend");
    let view = view_with(PropValue::Str("Hello".to_string()), vec![]);
    let scene = SceneGraph::from_view(&view);
    let (w, h) = (128u32, 40u32);
    let pixels = backend.render_offscreen(&scene, w, h).expect("offscreen");
    // 文本区域应存在非背景像素（验证文本路径接入，非占位矩形）
    let has_non_bg = pixels
        .chunks_exact(4)
        .any(|px| px[0] > 20 || px[1] > 20 || px[2] > 20);
    assert!(has_non_bg, "文本区应有非背景像素");
}

#[test]
fn e2e3_multi_node_layout_pixels() {
    let mut backend = VelloBackend::new().expect("vello backend");
    // 容器 + 2 子（不同颜色），布局后各子区域中心应落各自颜色
    let view = view_with(
        PropValue::Unit,
        vec![
            view_with(PropValue::Color(Color::rgb(255, 0, 0)), vec![]),
            view_with(PropValue::Color(Color::rgb(0, 255, 0)), vec![]),
        ],
    );
    let scene = SceneGraph::from_view(&view);
    let (w, h) = (200u32, 200u32);
    let pixels = backend.render_offscreen(&scene, w, h).expect("offscreen");
    // 验证两个不同区域出现两种颜色（布局真实作用）
    let red_at = pixels.chunks_exact(4).find(|px| px[0] > 180 && px[1] < 80).is_some();
    let green_at = pixels.chunks_exact(4).find(|px| px[1] > 180 && px[0] < 80).is_some();
    assert!(red_at, "应渲染出红色子节点");
    assert!(green_at, "应渲染出绿色子节点");
}

#[test]
fn e2e4_background_transparent_or_default() {
    let mut backend = VelloBackend::new().expect("vello backend");
    let view = WidgetView::<M>::empty();
    let scene = SceneGraph::from_view(&view);
    let (w, h) = (32u32, 32u32);
    let pixels = backend.render_offscreen(&scene, w, h).expect("offscreen");
    assert_eq!(pixels.len(), (w * h * 4) as usize); // 无 panic
}

// ============ 契约探测：RenderBackend 单一 vello（greenfield §B.2） ============

#[test]
fn render_backend_single_vello() {
    let _b = RenderBackend::Vello(VelloBackend::new().expect("vello backend"));
}
