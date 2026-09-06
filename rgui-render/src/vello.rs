//! vello 渲染后端——**单一后端**（greenfield §B.2：`RenderBackend` 仅 `Vello` 变体，
//! 无 skia/多后端抽象）。
//!
//! `VelloBackend` 用 wgpu 无窗口设备做**离屏渲染**：把 `SceneGraph` 的纯 Rust 绘制指令
//! 转成 vello `Scene`，渲染到 RGBA8 纹理，再读回 CPU 像素 buffer——证明"能画出来"。
//!
//! D5 范围：离屏渲染（矩形/文本原语可像素验证）。窗口/表面集成留后续。

use crate::scene_graph::{DrawCmd, SceneGraph};
use rgui_core::view::Color;
use vello::peniko::{self, Brush, Fill};
use vello::{kurbo, AaConfig, AaSupport, Renderer, RendererOptions, Scene};

/// 渲染后端（greenfield §B.2：单一 vello 变体）。
pub enum RenderBackend {
    /// vello 后端（唯一）。
    Vello(VelloBackend),
}

/// vello 离屏后端。
pub struct VelloBackend {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    /// 文本整形器（cosmic-text 真实字形）。
    text: crate::text::TextShaper,
}

impl VelloBackend {
    /// 创建 wgpu 无窗口设备 + vello renderer（离屏）。
    pub fn new() -> Result<Self, String> {
        tracing::info!(target: "rgui_render", "vello_init");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("no adapter: {e}"))?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|e| format!("no device: {e}"))?;

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::all(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        )
        .map_err(|e| format!("vello renderer: {e}"))?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            renderer,
            text: crate::text::TextShaper::new(),
        })
    }

    /// 把 `SceneGraph` 渲染到给定纹理视图（共享核心；离屏/窗口 surface 复用）。
    ///
    /// `scale`：逻辑→物理像素缩放（D17 渲染尺寸统一；离屏传 1.0，surface 传窗口 scale_factor）。
    pub fn render_to_view(
        &mut self,
        graph: &SceneGraph,
        view: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Result<(), String> {
        let mut scene = Scene::new();
        self.encode(&mut scene, graph, scale);
        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                &scene,
                view,
                &vello::RenderParams {
                    base_color: peniko::Color::new([0.0, 0.0, 0.0, 1.0]),
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .map_err(|e| format!("render_to_texture: {e}"))
    }

    /// 底层 wgpu 设备（供 facade 配置 surface）。
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// 底层 wgpu 适配器（供 facade 配置 surface）。
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// 底层 wgpu 队列（供 facade 提交 surface）。
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// 为窗口创建 wgpu surface（D7：把离屏扩展到窗口表面渲染）。
    pub fn create_surface<'window>(
        &self,
        window: impl Into<wgpu::SurfaceTarget<'window>>,
    ) -> Result<wgpu::Surface<'window>, String> {
        self.instance
            .create_surface(window)
            .map_err(|e| format!("create_surface: {e}"))
    }

    /// 创建 vello 离屏/中间渲染纹理（D7：供 render_to_view 渲染，再 blit 到 surface）。
    pub fn create_offscreen_texture(&self, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello_offscreen_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// 离屏渲染：把 `SceneGraph` 绘制指令渲染到 RGBA 像素 buffer（无窗口）。
    ///
    /// 返回紧致 RGBA8（每像素 4 字节，行紧密，无 256 对齐）CPU buffer。
    pub fn render_offscreen(
        &mut self,
        graph: &SceneGraph,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String> {
        // 目标纹理：Rgba8Unorm + STORAGE_BINDING（vello 要求）+ COPY_SRC（供 readback）
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello_offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.render_to_view(graph, &view, width, height, 1.0)?;

        // 读回 buffer：COPY_DST | MAP_READ，bytes_per_row 对齐 256
        let bytes_per_row = width * 4;
        let aligned = (bytes_per_row + 255) & !255;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vello_readback"),
            size: (aligned * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // copy texture → buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // 读回（map + poll）
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        let res = rx.recv().map_err(|e| format!("map recv: {e}"))?;
        res.map_err(|e| format!("map: {e}"))?;

        let data = slice.get_mapped_range();
        // 去对齐（紧密像素）
        let mut pixels = Vec::with_capacity((bytes_per_row * height) as usize);
        for row in 0..height {
            let start = (row * aligned) as usize;
            let end = start + bytes_per_row as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        buffer.unmap();
        Ok(pixels)
    }

    /// 把 `SceneGraph` 渲染到窗口 surface 并呈现（D8 封装：平台/上层不直接碰 wgpu）。
    ///
    /// 内部：configure surface（RENDER_ATTACHMENT）→ 离屏纹理（STORAGE_BINDING）→
    /// vello 渲染 → TextureBlitter blit 到 surface → present。
    pub fn render_surface(
        &mut self,
        surface: &wgpu::Surface<'_>,
        graph: &SceneGraph,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        let config = surface
            .get_default_config(self.adapter(), width, height)
            .unwrap_or_else(|| wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface.get_capabilities(self.adapter()).formats[0],
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            });
        surface.configure(&self.device, &config);
        let format = config.format;

        // 离屏中间纹理（STORAGE_BINDING + TEXTURE_BINDING + COPY_SRC）
        let offscreen = self.create_offscreen_texture(width, height);
        let off_view = offscreen.create_view(&wgpu::TextureViewDescriptor::default());
        self.render_to_view(graph, &off_view, width, height, scale)?;

        // 获取帧 + blit 呈现
        let surface_texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return Ok(()),
        };
        let surf_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let blitter = wgpu::util::TextureBlitter::new(&self.device, format);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        blitter.copy(&self.device, &mut encoder, &off_view, &surf_view);
        self.queue.submit(std::iter::once(encoder.finish()));
        let _ = surface_texture.present();
        Ok(())
    }

    /// 把 `SceneGraph` 的绘制指令编码为 vello `Scene`。文本用真实字形（cosmic-text）。
    /// `scale`：逻辑→物理像素缩放（D17 渲染尺寸统一）。
    fn encode(&mut self, scene: &mut Scene, graph: &SceneGraph, scale: f64) {
        let tf = kurbo::Affine::scale(scale);
        for cmd in graph.cmds() {
            match cmd {
                DrawCmd::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => {
                    let rect = kurbo::Rect::new(
                        f64::from(*x),
                        f64::from(*y),
                        f64::from(*x + *width),
                        f64::from(*y + *height),
                    );
                    let brush = Brush::Solid(to_peniko_color(*color));
                    scene.fill(Fill::NonZero, tf, brush, None, &rect);
                }
                DrawCmd::DrawText {
                    x,
                    y,
                    text,
                    size,
                    color,
                    width,
                } => {
                    self.draw_text(scene, *x, *y, text, *size, *color, *width, tf);
                }
                DrawCmd::StrokeRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                    stroke_width,
                } => {
                    // D16：描边矩形（vello `stroke`）——获焦边框高亮
                    let rect = kurbo::Rect::new(
                        f64::from(*x),
                        f64::from(*y),
                        f64::from(*x + *width),
                        f64::from(*y + *height),
                    );
                    let stroke = kurbo::Stroke::new(f64::from(*stroke_width));
                    let brush = Brush::Solid(to_peniko_color(*color));
                    scene.stroke(&stroke, tf, brush, None, &rect);
                }
            }
        }
    }

    /// 用真实字形绘制文本（cosmic-text 整形 → vello draw_glyphs）。`width`>0 时按宽度换行。无字形则矩形兜底。
    /// `tf`：逻辑→物理缩放变换。
    fn draw_text(
        &mut self,
        scene: &mut Scene,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        width: f32,
        tf: kurbo::Affine,
    ) {
        let brush = Brush::Solid(to_peniko_color(color));
        let runs = self
            .text
            .shape_line(text, size, if width > 0.0 { Some(width) } else { None });
        if runs.is_empty() {
            // 兜底：无字体/空文本时画一个近似块（不至于完全空）。
            let rect_width = text.chars().count() as f32 * size * 0.6;
            let rect = kurbo::Rect::new(
                f64::from(x),
                f64::from(y),
                f64::from(x + rect_width),
                f64::from(y + size),
            );
            scene.fill(Fill::NonZero, tf, brush, None, &rect);
            return;
        }
        for run in &runs {
            // glyph 坐标为相对 run 原点（无 x/y），先 translate 到 (x,y) 再整体 scale 放大
            let run_tf = tf * kurbo::Affine::translate((f64::from(x), f64::from(y)));
            scene
                .draw_glyphs(&run.font_data)
                .transform(run_tf)
                .font_size(size)
                .brush(brush.clone())
                .draw(Fill::NonZero, run.glyphs.iter().copied());
        }
    }
}
fn to_peniko_color(c: Color) -> peniko::Color {
    peniko::Color::from_rgba8(c.r, c.g, c.b, c.a)
}
