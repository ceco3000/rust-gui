//! 渲染上下文——Vello 矢量渲染 + blit pipeline。
//!
//! 委托给 `rgui_render::VelloBackend`，本模块仅提供 facade 层的
//! 便捷封装，保持与旧版 API 的兼容。

use std::sync::Arc;

use rgui_render::{RenderBackend, RenderError, RenderParams, SceneGraph};
pub use rgui_render::{VelloBackend, encode_scene_to_vello};

/// Vello 渲染上下文——管理 GPU 设备、表面和帧循环。
///
/// 封装 `rgui_render::VelloBackend`，提供简化的 `render()` 方法
/// 完成完整帧提交。使用双阶段管线：
/// `SceneGraph → RGBA target → blitter → BGRA surface → present`
pub struct RenderContext {
    /// 底层 Vello 渲染后端
    backend: VelloBackend,
}

impl RenderContext {
    /// 创建新的 RenderContext。
    ///
    /// # 参数
    ///
    /// * `window` - winit 窗口的 Arc 引用。
    ///
    /// # 错误
    ///
    /// 如果表面创建或渲染器初始化失败，返回 `RenderError`。
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, RenderError> {
        let size = window.inner_size();
        let w = size.width;
        let h = size.height;
        let backend = VelloBackend::new(window, w, h)?;
        Ok(Self { backend })
    }

    /// 调整渲染表面尺寸。
    pub fn resize(&mut self, w: u32, h: u32) {
        self.backend.resize(w, h);
    }

    /// Vello 渲染一帧：构建场景图 → 调用后端渲染 → 提交屏幕
    pub fn render(&mut self) -> Result<(), RenderError> {
        let (w, h) = self.backend.dimensions();
        let scene = SceneGraph::new(self.backend.frame_count());
        let params = RenderParams {
            width: w,
            height: h,
            clear_color: Some(rgui_core::Color::new(
                14.0 / 255.0,
                18.0 / 255.0,
                28.0 / 255.0,
                1.0,
            )),
            ..Default::default()
        };
        self.backend.render(&scene, &params)
    }

    /// 返回已渲染帧数。
    pub fn frame_count(&self) -> u64 {
        self.backend.frame_count()
    }

    /// 获取底层 VelloBackend 引用，供需要直接访问 RenderBackend trait 的场景使用。
    #[must_use]
    pub fn backend(&self) -> &VelloBackend {
        &self.backend
    }

    /// 获取底层 VelloBackend 的可变引用。
    pub fn backend_mut(&mut self) -> &mut VelloBackend {
        &mut self.backend
    }
}

impl std::fmt::Debug for RenderContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (w, h) = self.backend.dimensions();
        f.debug_struct("RenderContext(Vello+blit)")
            .field("backend", &self.backend.backend_name())
            .field("size", &format!("{w}x{h}"))
            .field("frames", &self.backend.frame_count())
            .finish()
    }
}
