//! 离屏渲染测试运行器（D9 §4）。
//!
//! 不依赖窗口系统和 GPU 显示输出的渲染测试器。
//! 使用 Skia CPU 光栅化作为默认后端，适用于无 GPU 环境的 CI 测试和
//! 截图回归验证。
//!
//! # 示例
//!
//! ```ignore
//! use rgui_render::offscreen::OffscreenTestRunner;
//! use rgui_render::scene::SceneGraph;
//!
//! let mut runner = OffscreenTestRunner::new(100, 100);
//! let scene = SceneGraph::new(1);
//! let pixels = runner.render(&scene).unwrap();
//! assert_eq!(pixels.len(), 100 * 100 * 4);
//! ```

#[cfg(feature = "offscreen")]
use std::path::Path;

use crate::backend::{RenderBackend, RenderError, RenderParams};
use crate::scene::SceneGraph;
use crate::skia::SkiaBackend;

// ============================================================================
// OffscreenError
// ============================================================================

/// 离屏渲染错误类型。
#[derive(Debug, thiserror::Error)]
pub enum OffscreenError {
    /// 渲染后端错误。
    #[error("渲染失败：{0}")]
    Render(#[from] RenderError),

    /// 图像缓冲区创建失败（尺寸为零或像素数据不匹配）。
    #[error("图像创建失败：像素数据大小与尺寸不匹配")]
    ImageCreationFailed,

    /// PNG 编码错误（仅在 `offscreen` feature 下可用）。
    #[cfg(feature = "offscreen")]
    #[error("PNG 编码失败：{0}")]
    PngEncode(#[from] image::ImageError),
}

// ============================================================================
// OffscreenTestRunner
// ============================================================================

/// 离屏渲染测试运行器（D9 §4）。
///
/// 封装 [`SkiaBackend`]（CPU 光栅化），提供简化的离屏渲染 API：
/// - [`render`](OffscreenTestRunner::render) 返回 RGBA 像素缓冲区
/// - [`render_to_png`](OffscreenTestRunner::render_to_png) 保存为 PNG 文件（需 `offscreen` feature）
///
/// # 设计
///
/// 当前使用 Skia CPU 光栅化作为默认后端。Vello 后端迁移到 `rgui-render` 后
/// （任务 R05），将增加 wgpu 离屏渲染路径。
pub struct OffscreenTestRunner {
    /// CPU 渲染后端。
    backend: SkiaBackend,
    /// 渲染表面宽度（像素单位）。
    width: u32,
    /// 渲染表面高度（像素单位）。
    height: u32,
}

impl OffscreenTestRunner {
    /// 创建新的离屏渲染测试运行器。
    ///
    /// 尺寸至少为 1 像素（与 [`SkiaBackend`] 行为一致）。
    /// 默认使用 Skia CPU 光栅化后端。
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            backend: SkiaBackend::new(),
            width: width.max(1),
            height: height.max(1),
        }
    }

    /// 使用指定的 [`SkiaBackend`] 实例创建运行器。
    ///
    /// 尺寸至少为 1 像素。用于需要自定义后端状态的测试场景。
    #[must_use]
    pub fn with_backend(backend: SkiaBackend, width: u32, height: u32) -> Self {
        Self {
            backend,
            width: width.max(1),
            height: height.max(1),
        }
    }

    /// 渲染场景图到离屏缓冲区，返回 RGBA 像素数据。
    ///
    /// 返回的缓冲区长度为 `width * height * 4`，像素格式为 RGBA8888，
    /// 行优先存储，无行填充。
    ///
    /// # 错误
    ///
    /// 如果渲染失败（如非法场景图结构），返回 [`RenderError`]。
    pub fn render(&mut self, scene: &SceneGraph) -> Result<Vec<u8>, RenderError> {
        let params = RenderParams {
            width: self.width,
            height: self.height,
            clear_color: Some(rgui_core::Color::new(1.0, 1.0, 1.0, 1.0)),
            ..RenderParams::default()
        };
        self.backend.render(scene, &params)?;
        Ok(self.backend.pixels().to_vec())
    }

    /// 渲染场景图并保存为 PNG 文件。
    ///
    /// 仅在 `offscreen` feature 下可用（需要 `image` crate）。
    ///
    /// # 错误
    ///
    /// - [`OffscreenError::Render`]：渲染失败
    /// - [`OffscreenError::ImageCreationFailed`]：像素数据与尺寸不匹配
    /// - [`OffscreenError::PngEncode`]：PNG 编码失败（`image` crate 错误）
    #[cfg(feature = "offscreen")]
    pub fn render_to_png(&mut self, scene: &SceneGraph, path: &Path) -> Result<(), OffscreenError> {
        let pixels = self.render(scene)?;

        let img = image::RgbaImage::from_raw(self.width, self.height, pixels)
            .ok_or(OffscreenError::ImageCreationFailed)?;
        img.save(path)?;
        Ok(())
    }

    /// 返回当前渲染表面尺寸（宽度，高度）。
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// 调整渲染表面尺寸。
    ///
    /// 尺寸至少 clamp 为 1 像素，确保与 [`dimensions`](OffscreenTestRunner::dimensions)
    /// 返回值一致。下次调用 [`render`](OffscreenTestRunner::render) 时将使用新尺寸。
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
    }

    /// 返回对内部渲染后端的引用（用于检查后端状态）。
    #[must_use]
    pub fn backend(&self) -> &SkiaBackend {
        &self.backend
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{FillRule, Paint, PathCommand, PathData};
    use crate::scene::{DrawCommand, SceneLayer};
    use rgui_core::geometry::Rect;
    use rgui_core::id::WidgetId;

    /// 创建一个包含单个 FillRect 命令的测试场景。
    fn test_scene() -> SceneGraph {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(10.0, 10.0, 50.0, 50.0),
            color: rgui_core::Color::RED,
            radius: 0.0,
        });
        SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        }
    }

    #[test]
    fn new_creates_runner_with_correct_dimensions() {
        let runner = OffscreenTestRunner::new(200, 100);
        assert_eq!(runner.dimensions(), (200, 100));
    }

    #[test]
    fn with_backend_preserves_custom_state() {
        let mut backend = SkiaBackend::new();
        // 预先注册一个纹理
        let tex_data = crate::texture::TextureData {
            width: 2,
            height: 2,
            pixels: vec![
                255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            format: crate::texture::TextureFormat::Rgba8,
        };
        backend.register_texture(&tex_data, crate::texture::TextureFormat::Rgba8);

        let runner = OffscreenTestRunner::with_backend(backend, 64, 64);
        assert_eq!(runner.dimensions(), (64, 64));
        assert_eq!(runner.backend().backend_name(), "Skia (CPU)");
    }

    #[test]
    fn render_returns_correct_buffer_size() {
        let mut runner = OffscreenTestRunner::new(100, 50);
        let scene = SceneGraph::new(1);
        let pixels = runner.render(&scene).expect("渲染应成功");
        assert_eq!(pixels.len(), 100 * 50 * 4);
    }

    #[test]
    fn render_empty_scene_is_white() {
        let mut runner = OffscreenTestRunner::new(20, 20);
        let scene = SceneGraph::new(1);
        let pixels = runner.render(&scene).expect("渲染应成功");

        // 检查中心像素应为白色（清除颜色 #FFFFFFFF）
        let center = (10 * 20 * 4 + 10 * 4) as usize;
        assert_eq!(pixels[center], 255); // R
        assert_eq!(pixels[center + 1], 255); // G
        assert_eq!(pixels[center + 2], 255); // B
        assert_eq!(pixels[center + 3], 255); // A
    }

    #[test]
    fn render_filled_rect_contains_color() {
        let mut runner = OffscreenTestRunner::new(100, 100);
        let scene = test_scene();
        let pixels = runner.render(&scene).expect("渲染应成功");

        // 红色矩形在 (10, 10, 50, 50)，中心点 (35, 35) 应为红色
        let center = (35 * 100 * 4 + 35 * 4) as usize;
        assert!(pixels[center] > 200, "红色矩形的 R 通道应 > 200");
        // 绿色和蓝色通道在红色矩形内应较低
        assert!(pixels[center + 1] < 100, "红色矩形的 G 通道应较低");
        assert!(pixels[center + 2] < 100, "红色矩形的 B 通道应较低");
    }

    #[test]
    fn render_rect_outside_bounds_is_white() {
        let mut runner = OffscreenTestRunner::new(100, 100);
        let scene = test_scene();
        let pixels = runner.render(&scene).expect("渲染应成功");

        // 矩形在 (10, 10, 50, 50)，区域外 (80, 80) 应为白色
        let outside = (80 * 100 * 4 + 80 * 4) as usize;
        assert_eq!(pixels[outside], 255); // R
        assert_eq!(pixels[outside + 1], 255); // G
        assert_eq!(pixels[outside + 2], 255); // B
    }

    #[test]
    fn resize_changes_dimensions() {
        let mut runner = OffscreenTestRunner::new(100, 100);
        assert_eq!(runner.dimensions(), (100, 100));

        runner.resize(200, 150);
        assert_eq!(runner.dimensions(), (200, 150));

        // 渲染后缓冲区大小应与新尺寸匹配
        let scene = SceneGraph::new(1);
        let pixels = runner.render(&scene).expect("渲染应成功");
        assert_eq!(pixels.len(), 200 * 150 * 4);
    }

    #[test]
    fn render_after_resize_uses_new_dimensions() {
        let mut runner = OffscreenTestRunner::new(10, 10);
        runner.resize(30, 20);

        let scene = SceneGraph::new(1);
        let pixels = runner.render(&scene).expect("渲染应成功");
        assert_eq!(pixels.len(), 30 * 20 * 4);
    }

    #[test]
    fn render_multiple_frames_consistent() {
        let mut runner = OffscreenTestRunner::new(50, 50);

        // 第一帧
        let scene = test_scene();
        let pixels1 = runner.render(&scene).expect("渲染应成功");

        // 第二帧相同场景
        let pixels2 = runner.render(&scene).expect("渲染应成功");

        assert_eq!(pixels1.len(), pixels2.len());
        // 相同场景应产生相同像素
        assert_eq!(pixels1, pixels2);
    }

    #[test]
    fn backend_accessor_returns_correct_name() {
        let runner = OffscreenTestRunner::new(64, 64);
        assert_eq!(runner.backend().backend_name(), "Skia (CPU)");
    }

    #[test]
    fn render_with_path_and_opacity() {
        let mut runner = OffscreenTestRunner::new(100, 100);

        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));

        // 半透明三角形路径
        let path = PathData {
            commands: vec![
                PathCommand::MoveTo { x: 50.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 90.0 },
                PathCommand::LineTo { x: 90.0, y: 90.0 },
                PathCommand::Close,
            ],
            fill_rule: FillRule::NonZero,
        };

        layer.push(DrawCommand::PushOpacity { opacity: 0.8 });
        layer.push(DrawCommand::FillPath {
            path,
            paint: Paint::Solid(rgui_core::Color::BLUE),
        });
        layer.push(DrawCommand::PopOpacity);

        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };

        let pixels = runner.render(&scene).expect("渲染应成功");

        // 三角形内部 (50, 50) 应有蓝色分量
        let inside = (50 * 100 * 4 + 50 * 4) as usize;
        assert!(pixels[inside + 2] > 100, "半透明蓝色三角形应有蓝色分量");
    }

    #[test]
    fn render_zero_size_does_not_panic() {
        let mut runner = OffscreenTestRunner::new(1, 1);
        // 即使 scene 为空，render 也应该正常工作（SkiaBackend 内部处理 min(1,1)）
        let scene = SceneGraph::new(1);
        let result = runner.render(&scene);
        assert!(result.is_ok());
    }

    #[test]
    fn render_with_clip_and_transform() {
        let mut runner = OffscreenTestRunner::new(100, 100);

        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));

        // 裁剪到 50x50 区域并变换
        layer.push(DrawCommand::PushClip {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
        });
        layer.push(DrawCommand::PushTransform {
            transform: crate::primitives::Transform::translate(5.0, 5.0),
        });
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 40.0, 40.0),
            color: rgui_core::Color::GREEN,
            radius: 0.0,
        });
        layer.push(DrawCommand::PopTransform);
        layer.push(DrawCommand::PopClip);

        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };

        let pixels = runner.render(&scene).expect("渲染应成功");

        // 变换后矩形应在 (5, 5)-(45, 45) 区域，中心有绿色
        let inside = (25 * 100 * 4 + 25 * 4) as usize;
        assert!(pixels[inside + 1] > 100, "裁剪+变换后的矩形应有绿色分量");

        // 裁剪区域外 (60, 60) 应为白色
        let outside = (60 * 100 * 4 + 60 * 4) as usize;
        assert_eq!(pixels[outside], 255);
        assert_eq!(pixels[outside + 1], 255);
        assert_eq!(pixels[outside + 2], 255);
    }

    // ============================================================================
    // 边界条件测试
    // ============================================================================

    #[test]
    fn render_with_max_dimensions() {
        // 大尺寸渲染（验证无 OOM panic）
        let mut runner = OffscreenTestRunner::new(500, 500);
        let scene = test_scene();
        let result = runner.render(&scene);
        assert!(result.is_ok());
    }

    #[test]
    fn resize_to_zero_clamps_to_one() {
        let mut runner = OffscreenTestRunner::new(100, 100);
        runner.resize(0, 0);
        // 尺寸被 clamp 到至少 1
        assert_eq!(runner.dimensions(), (1, 1));
        let scene = SceneGraph::new(1);
        let result = runner.render(&scene);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 4); // 1x1x4
    }

    #[test]
    fn consecutive_renders_incrementing_size() {
        let mut runner = OffscreenTestRunner::new(16, 16);

        for size in &[32u32, 64, 128] {
            runner.resize(*size, *size);
            let scene = SceneGraph::new(1);
            let pixels = runner.render(&scene).expect("渲染应成功");
            assert_eq!(pixels.len(), (*size * *size * 4) as usize);
        }
    }
}
