//! 渲染上下文——Vello 矢量渲染 + blit pipeline。
//!
//! 使用 verify/v1-vello-cosmic 验证过的模式：
//! Vello Scene → RGBA target → blitter → BGRA surface → present

use std::sync::Arc;

/// Vello 渲染上下文——管理 GPU 设备、表面和帧循环。
///
/// 使用 Vello 矢量渲染管线：
/// `Scene → RGBA target → blitter → BGRA surface → present`
///
/// 封装 `vello::util::RenderContext` 和 `vello::Renderer`，
/// 提供简化的 `render()` 方法完成完整帧提交。
pub struct RenderContext {
    /// Vello 渲染上下文（管理 GPU 设备 + 表面）
    gpu: vello::util::RenderContext,
    /// Vello 渲染表面（含 target_view + blitter）
    surface: vello::util::RenderSurface<'static>,
    /// Vello 矢量渲染器
    renderer: vello::Renderer,
    /// Vello 场景（每帧重建）
    scene: vello::Scene,
    frame_count: u64,
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
    /// 如果表面创建或渲染器初始化失败，返回错误信息。
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, String> {
        let mut gpu = vello::util::RenderContext::new();
        let size = window.inner_size();

        let surface = pollster::block_on(gpu.create_surface(
            window,
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        ))
        .map_err(|e| format!("surface: {e}"))?;

        let device_handle = &gpu.devices[surface.dev_id];
        let renderer =
            vello::Renderer::new(&device_handle.device, vello::RendererOptions::default())
                .map_err(|e| format!("vello: {e}"))?;

        Ok(Self {
            gpu,
            surface,
            renderer,
            scene: vello::Scene::new(),
            frame_count: 0,
        })
    }

    /// 调整渲染表面尺寸。
    pub fn resize(&mut self, w: u32, h: u32) {
        self.gpu.resize_surface(&mut self.surface, w, h);
    }

    /// 获取 Vello 场景的可变引用——在此编码形状/文字。
    pub fn scene_mut(&mut self) -> &mut vello::Scene {
        &mut self.scene
    }

    /// Vello 渲染一帧：scene → RGBA target → blit → BGRA surface → present
    pub fn render(&mut self) -> Result<(), String> {
        let device_handle = &self.gpu.devices[self.surface.dev_id];
        let device = &device_handle.device;
        let queue = &device_handle.queue;

        // 背景（深蓝灰色）
        let w = self.surface.config.width as f64;
        let h = self.surface.config.height as f64;
        self.scene.fill(
            vello::peniko::Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            vello::peniko::Color::from_rgba8(14, 18, 28, 255),
            None,
            &vello::kurbo::Rect::new(0.0, 0.0, w, h),
        );

        // 阶段 1: Vello 渲染到 RGBA target 纹理
        self.renderer
            .render_to_texture(
                device,
                queue,
                &self.scene,
                &self.surface.target_view,
                &vello::RenderParams {
                    base_color: vello::peniko::Color::TRANSPARENT,
                    width: self.surface.config.width,
                    height: self.surface.config.height,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .map_err(|e| format!("vello render: {e}"))?;

        // 阶段 2: Blit RGBA target → BGRA surface
        let surface_texture = self
            .surface
            .surface
            .get_current_texture()
            .map_err(|e| format!("surface: {e}"))?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("blit"),
        });
        self.surface.blitter.copy(
            device,
            &mut encoder,
            &self.surface.target_view,
            &surface_view,
        );
        queue.submit(Some(encoder.finish()));
        surface_texture.present();

        self.scene.reset();
        self.frame_count += 1;
        Ok(())
    }

    /// 返回已渲染帧数。
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl std::fmt::Debug for RenderContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderContext(Vello+blit)")
            .field(
                "size",
                &format!(
                    "{}x{}",
                    self.surface.config.width, self.surface.config.height
                ),
            )
            .field("frames", &self.frame_count)
            .finish()
    }
}
