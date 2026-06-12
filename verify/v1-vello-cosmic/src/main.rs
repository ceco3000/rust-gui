//! V1 验证：Vello + cosmic-text 协同渲染 POC
//!
//! 验证目标：cosmic-text 的字形布局数据能否与 Vello 场景图组合，
//! 在单帧内完成绘制。同时验证 SwashCache 字形光栅化 → wgpu 纹理 →
//! Vello ImageData 的完整管线。
//!
//! 设计文档：docs/Rust GUI 框架技术路线验证设计.md §V1

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use cosmic_text::{
    Attrs, Buffer, CacheKey, CacheKeyFlags, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use pollster::block_on;
use swash::scale::image::Content;
use vello::{
    kurbo::Affine,
    peniko::{Color, Fill, ImageData},
    util::RenderContext,
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// 窗口尺寸常量
const WINDOW_WIDTH: u32 = 900;
const WINDOW_HEIGHT: u32 = 300;

/// 字形纹理缓存条目
#[allow(dead_code)]
struct GlyphEntry {
    /// Vello 注册的图像句柄
    image: ImageData,
    /// 字形宽度（像素，保留用于字形图集优化）
    _width: f64,
    /// 字形高度（像素，保留用于字形图集优化）
    _height: f64,
    /// X 方向偏移（用于对齐字形基线位置）
    offset_x: f64,
    /// Y 方向偏移
    offset_y: f64,
}

/// 应用状态
struct App<'s> {
    render_context: RenderContext,
    render_surface: vello::util::RenderSurface<'s>,
    renderer: Renderer,
    font_system: FontSystem,
    buffer: Buffer,
    swash_cache: SwashCache,
    glyph_textures: HashMap<u16, GlyphEntry>,
    textures_initialized: bool,
    start_time: Instant,
    frame_count: u64,
}

impl<'s> App<'s> {
    fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        let mut render_context = RenderContext::new();

        let render_surface = block_on(render_context.create_surface(
            window.clone(),
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        ))
        .expect("无法创建渲染表面");

        let device_handle = &render_context.devices[render_surface.dev_id];
        let device = &device_handle.device;

        let renderer = Renderer::new(
            device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::all(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        )
        .expect("无法创建 Vello 渲染器");

        let mut font_system = FontSystem::new();

        let metrics = Metrics {
            font_size: 32.0,
            line_height: 48.0,
        };
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_size(
            &mut font_system,
            Some(WINDOW_WIDTH as f32 - 40.0),
            Some(WINDOW_HEIGHT as f32 - 20.0),
        );

        buffer.set_text(
            &mut font_system,
            "你好，世界！Hello, World!",
            &Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
            None,
        );

        let swash_cache = SwashCache::new();

        Self {
            render_context,
            render_surface,
            renderer,
            font_system,
            buffer,
            swash_cache,
            glyph_textures: HashMap::new(),
            textures_initialized: false,
            start_time: Instant::now(),
            frame_count: 0,
        }
    }

    /// 首次渲染前：光栅化所有字形并注册为 Vello 纹理
    fn init_glyph_textures(&mut self) {
        let device_handle = &self.render_context.devices[self.render_surface.dev_id];
        let device = &device_handle.device;
        let queue = &device_handle.queue;

        self.buffer
            .shape_until_scroll(&mut self.font_system, false);

        for run in self.buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let glyph_id = glyph.glyph_id;
                if self.glyph_textures.contains_key(&glyph_id) {
                    continue;
                }

                let (cache_key, _x_offset, _y_offset) = CacheKey::new(
                    glyph.font_id,
                    glyph_id,
                    glyph.font_size,
                    (glyph.x, glyph.y),
                    glyph.font_weight,
                    CacheKeyFlags::empty(),
                );

                let swash_image = self
                    .swash_cache
                    .get_image_uncached(&mut self.font_system, cache_key);

                let swash_image = match swash_image {
                    Some(img) => img,
                    None => continue,
                };

                let width = swash_image.placement.width as u32;
                let height = swash_image.placement.height as u32;
                if width == 0 || height == 0 {
                    continue;
                }

                let offset_x = swash_image.placement.left as f64;
                let offset_y = -swash_image.placement.top as f64;

                // 转换 SwashImage → RGBA8 像素缓冲区
                let pixels = match &swash_image.content {
                    Content::Mask => {
                        let mut rgba = vec![0u8; (width * height * 4) as usize];
                        for (i, &alpha) in swash_image.data.iter().enumerate() {
                            rgba[i * 4] = 255;
                            rgba[i * 4 + 1] = 255;
                            rgba[i * 4 + 2] = 255;
                            rgba[i * 4 + 3] = alpha;
                        }
                        rgba
                    }
                    Content::SubpixelMask => {
                        let mut rgba = vec![0u8; (width * height * 4) as usize];
                        for i in 0..(width * height) as usize {
                            let si = i * 3;
                            let di = i * 4;
                            rgba[di] = swash_image.data[si];
                            rgba[di + 1] = swash_image.data[si + 1];
                            rgba[di + 2] = swash_image.data[si + 2];
                            rgba[di + 3] = 255;
                        }
                        rgba
                    }
                    Content::Color => continue,
                };

                let texture_extent = wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                };

                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(&format!("glyph_{glyph_id}")),
                    size: texture_extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });

                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &pixels,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * width),
                        rows_per_image: Some(height),
                    },
                    texture_extent,
                );

                let image = self.renderer.register_texture(texture);

                log::debug!(
                    "注册字形: glyph_id={glyph_id}, size={width}x{height}, offset=({offset_x},{offset_y})"
                );

                self.glyph_textures.insert(
                    glyph_id,
                    GlyphEntry {
                        image,
                        _width: width as f64,
                        _height: height as f64,
                        offset_x,
                        offset_y,
                    },
                );
            }
        }

        self.textures_initialized = true;
        log::info!(
            "字形纹理初始化完成：共 {} 个唯一字形",
            self.glyph_textures.len()
        );
    }

    fn handle_resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.render_context
                .resize_surface(&mut self.render_surface, width, height);
            self.buffer.set_size(
                &mut self.font_system,
                Some(width as f32 - 40.0),
                Some(height as f32 - 20.0),
            );
        }
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 预先克隆 Device 和 Queue（wgpu 类型实现 Clone），
        // 避免 init_glyph_textures 的 &mut self 与不可变引用冲突
        let device_handle = &self.render_context.devices[self.render_surface.dev_id];
        let device = device_handle.device.clone();
        let queue = device_handle.queue.clone();

        if !self.textures_initialized {
            self.init_glyph_textures();
        }

        self.buffer
            .shape_until_scroll(&mut self.font_system, false);

        let mut scene = Scene::new();

        // 背景
        let bg_w = self.render_surface.config.width as f64;
        let bg_h = self.render_surface.config.height as f64;
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgba8(28, 30, 34, 255),
            None,
            &vello::kurbo::Rect::new(0.0, 0.0, bg_w, bg_h),
        );

        // 绘制每个字形
        for run in self.buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let entry = match self.glyph_textures.get(&glyph.glyph_id) {
                    Some(e) => e,
                    None => continue,
                };

                let x = glyph.x as f64 + entry.offset_x;
                let y = glyph.y as f64 + entry.offset_y;

                // 翻转 Y 轴（cosmic-text 使用向下增长，Vello 默认向上）
                let transform =
                    Affine::translate((x, y)) * Affine::scale_non_uniform(1.0, -1.0);
                scene.draw_image(&entry.image, transform);
            }
        }

        // Vello 渲染到中间纹理
        self.renderer.render_to_texture(
            &device,
            &queue,
            &scene,
            &self.render_surface.target_view,
            &RenderParams {
                base_color: Color::TRANSPARENT,
                width: self.render_surface.config.width,
                height: self.render_surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        )?;

        // Blit 到表面
        let surface_texture = self
            .render_surface
            .surface
            .get_current_texture()
            .expect("无法获取表面纹理");
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blit_encoder"),
            });
        self.render_surface.blitter.copy(
            &device,
            &mut encoder,
            &self.render_surface.target_view,
            &surface_view,
        );
        queue.submit([encoder.finish()]);
        surface_texture.present();

        // 帧率统计
        self.frame_count += 1;
        if self.frame_count % 60 == 0 {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            let fps = self.frame_count as f64 / elapsed;
            log::info!("帧率: {:.1} fps, 总帧数: {}", fps, self.frame_count);
        }

        Ok(())
    }
}

/// winit 应用处理器
struct V1App {
    app: Option<App<'static>>,
}

impl ApplicationHandler for V1App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("V1: Vello + cosmic-text 协同渲染验证")
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));

        let window = event_loop
            .create_window(window_attributes)
            .expect("无法创建窗口");

        self.app = Some(App::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                log::info!("窗口关闭，退出验证");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = app.render() {
                    log::error!("渲染错误: {}", e);
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                app.handle_resize(size.width, size.height);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("=== V1 验证启动 ===");
    log::info!("验证目标：Vello + cosmic-text + SwashCache 字形纹理渲染");

    let event_loop = EventLoop::new().expect("无法创建 EventLoop");
    let mut app = V1App { app: None };

    event_loop
        .run_app(&mut app)
        .expect("事件循环异常退出");

    log::info!("=== V1 验证结束 ===");
}
// touch
