//! D5 验收测试 · 离屏渲染像素读回（B 层，GPU/软渲染依赖）
//!
//! 注入：cp tools/qa/d5_tests/d5_acceptance_offscreen.rs rgui-render/tests/
//! 运行：cargo test -p rgui-render --features vello-backend --test d5_acceptance_offscreen
//!
//! ⚠️ 环境依赖：本文件测试 vello 离屏渲染到 CPU 图像并像素读回，需要 wgpu/vello
//! 成功初始化（GPU 或软件适配器）。若环境不可用，这些测试标记 #[ignore] 或按环境跳过，
//! 并在验收报告注明"像素读回依赖 GPU 环境"。
//!
//! ⚠️ 契约锁定：`VelloBackend::render_offscreen`/`CpuImage` 为 dev D5 实现后应暴露的 API。
//! 当前 VelloBackend 为占位空壳，故本文件在 dev 实现离屏渲染前不注入。

use rgui_render::vello::VelloBackend;
use rgui_render::scene_graph::{SceneGraph, SceneNode};

/// 契约探测：离屏渲染 API（dev 实现后据此实现，本文件在实现后才编译）。
/// 说明：API 名以 dev 实际为准；此处锁定的契约形态为：
///   `VelloBackend::render_offscreen(&scene, width, height) -> CpuImage`
///   其中 `CpuImage` 提供像素采样（如 `pixels()` / `get_pixel(x,y)` / `as_raw()`）。

// ============ PX: 离屏渲染像素读回（契约锁定用例，环境就绪后运行） ============

// 以下用例在 dev 暴露 render_offscreen + CpuImage 后启用（见 D5 清单 §3 PX1-PX6）：
//   PX1 空场景渲染尺寸一致；PX2 矩形中心像素==指定色；PX3 文本区非空像素；
//   PX4 背景透明/默认；PX5 尺寸边界 0；PX6 颜色容差。

/// 环境探测：渲染后端是否可用（GPU/软适配器初始化）。
#[test]
#[ignore = "需 GPU/软渲染环境 + dev 实现 render_offscreen"]
fn offscreen_renders_red_rect() {
    let backend = VelloBackend::new();
    let mut scene = SceneGraph::new();
    scene.root = Some(SceneNode::default());
    // 契约示例：渲染 100x100，红色矩形覆盖中心
    let img = backend.render_offscreen(&scene, 100, 100);
    // 采样矩形中心像素应为红色（P2-style 断言，dev 实现后按实际 API 补）
    let _center = img.get_pixel(50, 50);
    // assert_eq!(center, /* 红色 */);
}

/// 文本渲染：文本覆盖区应有非透明像素（P3-style）。
#[test]
#[ignore = "需 GPU/软渲染环境 + dev 实现 render_offscreen"]
fn offscreen_text_produces_pixels() {
    let backend = VelloBackend::new();
    let scene = SceneGraph::new();
    let img = backend.render_offscreen(&scene, 100, 50);
    let _ = img; // dev 实现后补文本 draw + 像素断言
}

/// 尺寸边界：0 尺寸不应 panic（P5-style）。
#[test]
#[ignore = "需 dev 实现 render_offscreen"]
fn offscreen_zero_size_no_panic() {
    let backend = VelloBackend::new();
    let scene = SceneGraph::new();
    let img = backend.render_offscreen(&scene, 0, 0);
    let _ = img;
}
