//! 离屏渲染集成测试（TDD RED 起点）。
//!
//! 目标：`VelloBackend::render_offscreen` 用单一 vello 后端把一张 `Scene` 渲染到 RGBA
//! 像素 buffer（无窗口），可读回像素验证——证明"能画出来"。
//!
//! 本测试渲染一个填满画布的红色矩形，读回中心像素应为红色。

#![cfg(feature = "vello-backend")]

use rgui_render::vello::{RenderBackend, VelloBackend};
use rgui_render::scene_graph::SceneGraph;

#[test]
fn offscreen_renders_red_rect_to_pixels() {
    let mut backend = VelloBackend::new().expect("creates wgpu vello backend");

    let scene = SceneGraph::red_filled_rect(64.0, 64.0);
    let pixels = backend
        .render_offscreen(&scene, 64, 64)
        .expect("offscreen render must succeed");

    assert_eq!(pixels.len(), 64 * 64 * 4, "RGBA pixel buffer size");

    // 读回中心像素（应为纯红）
    let center = (32 * 64 + 32) * 4; // (y*width + x) * 4
    let r = pixels[center];
    let g = pixels[center + 1];
    let b = pixels[center + 2];
    assert!(r > 200, "R channel should be high, got {r}");
    assert!(g < 60, "G channel should be low, got {g}");
    assert!(b < 60, "B channel should be low, got {b}");
}

#[test]
fn render_backend_is_single_vello_variant() {
    // greenfield §B.2：RenderBackend 仅 Vello 变体（单一后端，无 skia/多后端抽象）
    let _b = RenderBackend::Vello(VelloBackend::new().expect("vello backend"));
}
