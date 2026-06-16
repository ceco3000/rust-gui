//! Vello GPU 矢量渲染后端实现（D3 §7）。
//!
//! 使用 vello + wgpu 在 GPU 上渲染 SceneGraph。VelloBackend 实现
//! [`RenderBackend`](crate::RenderBackend) trait，是 rgui 框架的主推
//! 渲染后端。
//!
//! # 架构
//!
//! - 构造时需要窗口句柄（`HasWindowHandle + HasDisplayHandle`）以创建
//!   wgpu surface。
//! - `render()` 方法将 SceneGraph 编码为 vello::Scene，再通过
//!   `render_to_texture()` + blitter 提交到屏幕。
//! - 双阶段管线：Scene → RGBA target → blitter → BGRA surface → present
//!
//! # 依赖
//!
//! 需要 `vello-backend` feature 启用（默认开启）。

use std::collections::HashMap;
use std::sync::Mutex;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::backend::{RenderBackend, RenderError, RenderParams};
use crate::primitives::{
    FillRule, LineCap, LineJoin, Paint, PathCommand, PathData, Stroke, Transform,
};
use crate::scene::{DrawCommand, SceneGraph};
use crate::texture::{TextureData, TextureFormat, TextureId};
use rgui_core::Color;

// ============================================================================
// VelloBackend
// ============================================================================

/// Vello GPU 矢量渲染后端（D3 §7）。
///
/// 使用 vello + wgpu 完成 GPU 加速矢量渲染，是 rgui 框架的主推后端。
///
/// # 双阶段渲染管线
///
/// ```text
/// SceneGraph → vello::Scene → RGBA target texture → blitter → BGRA surface → present
/// ```
pub struct VelloBackend {
    /// Vello GPU 上下文（管理设备 + surface）
    gpu: vello::util::RenderContext,
    /// Vello 渲染表面（含 target_view + blitter）
    surface: vello::util::RenderSurface<'static>,
    /// Vello 矢量渲染器（Mutex 包装以满足 `Sync` 约束——
    /// `vello::Renderer` 内部含 `RefCell`，本身非 `Sync`）
    renderer: Mutex<vello::Renderer>,
    /// 已注册的纹理映射
    textures: HashMap<TextureId, TextureData>,
    /// 纹理 ID 计数器
    next_texture_id: u64,
    /// 累计渲染帧数
    frame_count: u64,
    /// 当前表面宽度
    width: u32,
    /// 当前表面高度
    height: u32,
    /// 缓存的字形 atlas 数据（当帧有效）(width, height, RGBA8 pixels)
    atlas_cache: Option<(u32, u32, Vec<u8>)>,
}

impl VelloBackend {
    /// 创建新的 VelloBackend。
    ///
    /// # 参数
    ///
    /// * `window` — 窗口句柄，实现 `HasWindowHandle + HasDisplayHandle`。
    ///   通常为 `Arc<winit::window::Window>`。
    /// * `width` — 初始表面宽度（像素单位）。
    /// * `height` — 初始表面高度（像素单位）。
    ///
    /// # 错误
    ///
    /// - [`RenderError::SurfaceCreationFailed`] — 无法创建 wgpu surface。
    /// - [`RenderError::RenderFailed`] — 无法创建 Vello 渲染器。
    pub fn new(
        window: impl HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        let mut gpu = vello::util::RenderContext::new();

        let surface = pollster::block_on(gpu.create_surface(
            window,
            width,
            height,
            wgpu::PresentMode::AutoVsync,
        ))
        .map_err(|e| RenderError::SurfaceCreationFailed(format!("创建 wgpu surface 失败: {e}")))?;

        let device_handle = &gpu.devices[surface.dev_id];
        let renderer =
            vello::Renderer::new(&device_handle.device, vello::RendererOptions::default())
                .map_err(|e| RenderError::RenderFailed(format!("创建 Vello 渲染器失败: {e}")))?;

        Ok(Self {
            gpu,
            surface,
            renderer: Mutex::new(renderer),
            textures: HashMap::new(),
            next_texture_id: 1,
            frame_count: 0,
            width,
            height,
            atlas_cache: None,
        })
    }

    /// 调整渲染表面尺寸。
    pub fn resize(&mut self, width: u32, height: u32) {
        if width != self.width || height != self.height {
            self.gpu.resize_surface(&mut self.surface, width, height);
            self.width = width;
            self.height = height;
        }
    }

    /// 返回已渲染帧数。
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// 返回当前表面尺寸（像素单位）。
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// 设置当帧的字形 atlas 数据，用于 DrawGlyphs 真实纹理渲染。
    ///
    /// 在 `render()` 之前调用；数据在当帧渲染后被清除。
    pub fn set_atlas_data(&mut self, width: u32, height: u32, pixels: &[u8]) {
        if width > 0 && height > 0 && !pixels.is_empty() {
            self.atlas_cache = Some((width, height, pixels.to_vec()));
        }
    }

    /// 将 SceneGraph 编码为 vello::Scene（委托给独立函数）。
    fn encode_scene(&self, scene: &SceneGraph, params: &RenderParams) -> vello::Scene {
        encode_scene_to_vello(scene, params, self.atlas_cache.as_ref())
    }

    /// 执行 Vello 渲染管线。
    fn render_vello_scene(&mut self, vello_scene: &vello::Scene) -> Result<(), RenderError> {
        let device_handle = self.gpu.devices.get(self.surface.dev_id).ok_or_else(|| {
            RenderError::DeviceLost(format!(
                "内部状态不一致：设备 ID {} 越界（共 {} 个设备）",
                self.surface.dev_id,
                self.gpu.devices.len()
            ))
        })?;
        let device = &device_handle.device;
        let queue = &device_handle.queue;

        // 阶段 1: Vello 渲染到 RGBA target 纹理
        let mut renderer = self.renderer.lock().unwrap_or_else(|e| e.into_inner());
        renderer
            .render_to_texture(
                device,
                queue,
                vello_scene,
                &self.surface.target_view,
                &vello::RenderParams {
                    base_color: vello::peniko::Color::TRANSPARENT,
                    width: self.surface.config.width,
                    height: self.surface.config.height,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .map_err(|e| RenderError::RenderFailed(format!("Vello 渲染失败: {e}")))?;

        // 阶段 2: Blit RGBA target → BGRA surface
        let surface_texture = self
            .surface
            .surface
            .get_current_texture()
            .map_err(|e| RenderError::RenderFailed(format!("获取 surface 纹理失败: {e}")))?;

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

        Ok(())
    }
}

impl RenderBackend for VelloBackend {
    fn render(&mut self, scene: &SceneGraph, params: &RenderParams) -> Result<(), RenderError> {
        // 若尺寸变化，先调整表面
        if params.width != self.width || params.height != self.height {
            self.resize(params.width, params.height);
        }

        let vello_scene = self.encode_scene(scene, params);
        self.render_vello_scene(&vello_scene)?;

        // 清除当帧 atlas 数据（每帧数据由外部设置）
        self.atlas_cache = None;
        self.frame_count += 1;
        Ok(())
    }

    fn register_texture(&mut self, data: &TextureData, _format: TextureFormat) -> TextureId {
        let id = TextureId::new(self.next_texture_id);
        self.next_texture_id += 1;
        self.textures.insert(
            id,
            TextureData {
                width: data.width,
                height: data.height,
                pixels: data.pixels.clone(),
                format: data.format,
            },
        );
        id
    }

    fn unregister_texture(&mut self, id: TextureId) {
        self.textures.remove(&id);
    }

    fn backend_name(&self) -> &'static str {
        "Vello (GPU)"
    }
}

// ============================================================================
// SceneGraph → vello::Scene 编码（独立函数，可无 GPU 测试）
// ============================================================================

/// 将 SceneGraph 编码为 vello::Scene。
///
/// 此函数为纯数据转换，不依赖 GPU 设备，可在无 GPU 环境下测试。
///
/// `atlas_data` 为可选的 atlas 像素数据 `(width, height, RGBA8 pixels)`，
/// 传入时使用真实字形纹理渲染 `DrawGlyphs`；`None` 时回退到占位色块。
pub fn encode_scene_to_vello(
    scene: &SceneGraph,
    params: &RenderParams,
    atlas_data: Option<&(u32, u32, Vec<u8>)>,
) -> vello::Scene {
    let mut vello_scene = vello::Scene::new();

    // 填充背景色
    let w = params.width as f64;
    let h = params.height as f64;
    let bg_color = params.clear_color.unwrap_or(Color::new(1.0, 1.0, 1.0, 1.0));
    vello_scene.fill(
        vello::peniko::Fill::NonZero,
        vello::kurbo::Affine::IDENTITY,
        to_peniko_color(bg_color),
        None,
        &vello::kurbo::Rect::new(0.0, 0.0, w, h),
    );

    // 逐层编码
    for layer in &scene.layers {
        if layer.opacity <= 0.0 {
            continue;
        }

        // 层级别变换或透明度 → push_layer
        let needs_layer = layer.transform.is_some() || (layer.opacity - 1.0).abs() > f32::EPSILON;

        if needs_layer {
            let xform = layer
                .transform
                .as_ref()
                .map_or(vello::kurbo::Affine::IDENTITY, to_kurbo_affine);
            vello_scene.push_layer(
                vello::peniko::Fill::NonZero,
                vello::peniko::BlendMode::default(),
                layer.opacity,
                xform,
                &vello::kurbo::Rect::new(0.0, 0.0, w, h),
            );
        }

        for cmd in &layer.commands {
            encode_draw_command(&mut vello_scene, cmd, atlas_data);
        }

        if needs_layer {
            vello_scene.pop_layer();
        }
    }

    vello_scene
}

/// 将单个 DrawCommand 编码到 vello::Scene 中。
///
/// `atlas_data` 为可选的字形 Atlas 纹理数据 `(width, height, RGBA8 pixels)`，
/// 传入时使用真实字形纹理渲染 `DrawGlyphs`。
fn encode_draw_command(
    scene: &mut vello::Scene,
    cmd: &DrawCommand,
    atlas_data: Option<&(u32, u32, Vec<u8>)>,
) {
    match cmd {
        DrawCommand::FillRect {
            rect,
            color,
            radius,
        } => {
            let kurbo_rect = to_kurbo_rect(rect);
            if *radius > f32::EPSILON {
                let path = vello::kurbo::RoundedRect::new(
                    kurbo_rect.x0,
                    kurbo_rect.y0,
                    kurbo_rect.x1,
                    kurbo_rect.y1,
                    f64::from(*radius),
                );
                scene.fill(
                    vello::peniko::Fill::NonZero,
                    vello::kurbo::Affine::IDENTITY,
                    to_peniko_color(*color),
                    None,
                    &path,
                );
            } else {
                scene.fill(
                    vello::peniko::Fill::NonZero,
                    vello::kurbo::Affine::IDENTITY,
                    to_peniko_color(*color),
                    None,
                    &kurbo_rect,
                );
            }
        },
        DrawCommand::FillPath { path, paint } => {
            let kurbo_path = to_kurbo_path(path);
            let fill_rule = to_peniko_fill(path.fill_rule);
            match paint {
                Paint::Solid(color) => {
                    scene.fill(
                        fill_rule,
                        vello::kurbo::Affine::IDENTITY,
                        to_peniko_color(*color),
                        None,
                        &kurbo_path,
                    );
                },
                _ => {
                    // 渐变或图像着色器
                    if let Some(gradient) = to_peniko_gradient(paint) {
                        scene.fill(
                            fill_rule,
                            vello::kurbo::Affine::IDENTITY,
                            &gradient,
                            None,
                            &kurbo_path,
                        );
                    } else {
                        // 图像着色器占位：洋红色
                        scene.fill(
                            fill_rule,
                            vello::kurbo::Affine::IDENTITY,
                            vello::peniko::Color::from_rgba8(255, 0, 255, 128),
                            None,
                            &kurbo_path,
                        );
                    }
                },
            }
        },
        DrawCommand::StrokePath {
            path,
            stroke,
            paint,
        } => {
            let kurbo_path = to_kurbo_path(path);
            let kurbo_stroke = to_kurbo_stroke(stroke);
            match paint {
                Paint::Solid(color) => {
                    scene.stroke(
                        &kurbo_stroke,
                        vello::kurbo::Affine::IDENTITY,
                        to_peniko_color(*color),
                        None,
                        &kurbo_path,
                    );
                },
                _ => {
                    if let Some(gradient) = to_peniko_gradient(paint) {
                        scene.stroke(
                            &kurbo_stroke,
                            vello::kurbo::Affine::IDENTITY,
                            &gradient,
                            None,
                            &kurbo_path,
                        );
                    } else {
                        scene.stroke(
                            &kurbo_stroke,
                            vello::kurbo::Affine::IDENTITY,
                            vello::peniko::Color::from_rgba8(255, 0, 255, 128),
                            None,
                            &kurbo_path,
                        );
                    }
                },
            }
        },
        DrawCommand::DrawGlyphs {
            texture_id: _,
            glyphs,
            font_size,
            color,
        } => {
            if glyphs.is_empty() {
                return;
            }

            // 有 atlas 纹理数据时，渲染真实字形
            if let Some((atlas_w, atlas_h, atlas_pixels)) = atlas_data {
                let pixels: std::sync::Arc<dyn AsRef<[u8]> + Send + Sync> =
                    std::sync::Arc::new(atlas_pixels.clone());
                let image = vello::peniko::ImageData {
                    data: vello::peniko::Blob::new(pixels),
                    format: vello::peniko::ImageFormat::Rgba8,
                    alpha_type: vello::peniko::ImageAlphaType::Alpha,
                    width: *atlas_w,
                    height: *atlas_h,
                };

                for glyph in glyphs {
                    let gx = glyph.offset_x as f64;
                    let gy = glyph.offset_y as f64;
                    let gw = glyph.atlas_w as f64;
                    let gh = glyph.atlas_h as f64;
                    let au = glyph.atlas_x as f64;
                    let av = glyph.atlas_y as f64;

                    let glyph_xform = vello::kurbo::Affine::translate((gx, gy));
                    let brush_xform = vello::kurbo::Affine::translate((au, av));
                    let glyph_rect = vello::kurbo::Rect::new(0.0, 0.0, gw, gh);

                    let brush_ref: vello::peniko::ImageBrushRef =
                        vello::peniko::ImageBrushRef::from(&image);
                    let brush = vello::peniko::Brush::Image(brush_ref);

                    scene.fill(
                        vello::peniko::Fill::NonZero,
                        glyph_xform,
                        brush,
                        Some(brush_xform),
                        &glyph_rect,
                    );
                }
                return;
            }

            // 无 atlas 数据时回退到占位色块
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;

            for glyph in glyphs {
                let gx = glyph.offset_x;
                let gy = glyph.offset_y;
                let gw = glyph.atlas_w as f32;
                let gh = glyph.atlas_h as f32;
                min_x = min_x.min(gx);
                min_y = min_y.min(gy);
                max_x = max_x.max(gx + gw);
                max_y = max_y.max(gy + gh);
            }

            let padding = *font_size * 0.1;
            let text_rect = vello::kurbo::Rect::new(
                (min_x - padding).into(),
                (min_y - padding).into(),
                (max_x + padding).into(),
                (max_y + padding).into(),
            );

            let mut fill_color = to_peniko_color(*color);
            fill_color.components[3] = (fill_color.components[3] * 0.85).min(1.0);

            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                fill_color,
                None,
                &text_rect,
            );

            let baseline_y = min_y + *font_size;
            let baseline_start = vello::kurbo::Point::new(f64::from(min_x), f64::from(baseline_y));
            let baseline_end = vello::kurbo::Point::new(f64::from(max_x), f64::from(baseline_y));
            let baseline = vello::kurbo::Line::new(baseline_start, baseline_end);
            let stroke = vello::kurbo::Stroke::new(1.0);
            scene.stroke(
                &stroke,
                vello::kurbo::Affine::IDENTITY,
                to_peniko_color(Color::new(1.0, 1.0, 1.0, 0.6)),
                None,
                &baseline,
            );
        },
        DrawCommand::DrawImage {
            texture_id: _,
            src: _,
            dst,
            blend_mode: _,
        } => {
            // 图像渲染占位：洋红色半透明矩形
            let kurbo_rect = to_kurbo_rect(dst);
            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::from_rgba8(255, 0, 255, 128),
                None,
                &kurbo_rect,
            );
        },
        DrawCommand::PushClip { rect } => {
            let kurbo_rect = to_kurbo_rect(rect);
            scene.push_layer(
                vello::peniko::Fill::NonZero,
                vello::peniko::BlendMode::default(),
                1.0,
                vello::kurbo::Affine::IDENTITY,
                &kurbo_rect,
            );
        },
        DrawCommand::PopClip => {
            scene.pop_layer();
        },
        DrawCommand::PushTransform { transform } => {
            let big_clip = vello::kurbo::Rect::new(0.0, 0.0, 1e9, 1e9);
            scene.push_layer(
                vello::peniko::Fill::NonZero,
                vello::peniko::BlendMode::default(),
                1.0,
                to_kurbo_affine(transform),
                &big_clip,
            );
        },
        DrawCommand::PopTransform => {
            scene.pop_layer();
        },
        DrawCommand::PushOpacity { opacity } => {
            let big_clip = vello::kurbo::Rect::new(0.0, 0.0, 1e9, 1e9);
            scene.push_layer(
                vello::peniko::Fill::NonZero,
                vello::peniko::BlendMode::default(),
                *opacity,
                vello::kurbo::Affine::IDENTITY,
                &big_clip,
            );
        },
        DrawCommand::PopOpacity => {
            scene.pop_layer();
        },
    }
}

// ============================================================================
// 类型转换辅助函数
// ============================================================================

/// 将 rgui Color 转换为 vello peniko::Color（即 AlphaColor<Srgb>）。
fn to_peniko_color(color: Color) -> vello::peniko::Color {
    let [r, g, b, a] = color.to_u8_array();
    vello::peniko::Color::from_rgba8(r, g, b, a)
}

/// 将 rgui Color 转换为 peniko DynamicColor（用于 ColorStop）。
fn to_dynamic_color(color: Color) -> vello::peniko::color::DynamicColor {
    let pc = to_peniko_color(color);
    vello::peniko::color::DynamicColor::from_alpha_color(pc)
}

/// 将 rgui geometry::Rect 转换为 kurbo::Rect。
fn to_kurbo_rect(rect: &rgui_core::geometry::Rect) -> vello::kurbo::Rect {
    vello::kurbo::Rect::new(
        rect.origin.x,
        rect.origin.y,
        rect.origin.x + rect.size.width,
        rect.origin.y + rect.size.height,
    )
}

/// 将 rgui FillRule 转换为 peniko::Fill。
fn to_peniko_fill(fill_rule: FillRule) -> vello::peniko::Fill {
    match fill_rule {
        FillRule::NonZero => vello::peniko::Fill::NonZero,
        FillRule::EvenOdd => vello::peniko::Fill::EvenOdd,
    }
}

/// 将 rgui Paint 转换为 peniko::Gradient（仅渐变类型返回 Some，Solid 返回 None）。
fn to_peniko_gradient(paint: &Paint) -> Option<vello::peniko::Gradient> {
    match paint {
        Paint::Solid(_) => None,
        Paint::LinearGradient { start, end, stops } => {
            let peniko_stops_vec: Vec<vello::peniko::ColorStop> = stops
                .iter()
                .map(|s| vello::peniko::ColorStop {
                    offset: s.position,
                    color: to_dynamic_color(s.color),
                })
                .collect();
            let peniko_stops = vello::peniko::ColorStops::from(peniko_stops_vec.as_slice());
            let kind = vello::peniko::LinearGradientPosition {
                start: vello::kurbo::Point::new(start.x.into(), start.y.into()),
                end: vello::kurbo::Point::new(end.x.into(), end.y.into()),
            }
            .into();
            Some(vello::peniko::Gradient {
                kind,
                extend: vello::peniko::Extend::Pad,
                stops: peniko_stops,
                ..Default::default()
            })
        },
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => {
            let peniko_stops_vec: Vec<vello::peniko::ColorStop> = stops
                .iter()
                .map(|s| vello::peniko::ColorStop {
                    offset: s.position,
                    color: to_dynamic_color(s.color),
                })
                .collect();
            let peniko_stops = vello::peniko::ColorStops::from(peniko_stops_vec.as_slice());
            let kind = vello::peniko::RadialGradientPosition {
                start_center: vello::kurbo::Point::new(center.x.into(), center.y.into()),
                start_radius: 0.0,
                end_center: vello::kurbo::Point::new(center.x.into(), center.y.into()),
                end_radius: *radius,
            }
            .into();
            Some(vello::peniko::Gradient {
                kind,
                extend: vello::peniko::Extend::Pad,
                stops: peniko_stops,
                ..Default::default()
            })
        },
        Paint::Image { .. } => None,
    }
}

/// 将 rgui LineCap 转换为 kurbo::Cap。
fn to_kurbo_cap(cap: LineCap) -> vello::kurbo::Cap {
    match cap {
        LineCap::Butt => vello::kurbo::Cap::Butt,
        LineCap::Round => vello::kurbo::Cap::Round,
        LineCap::Square => vello::kurbo::Cap::Square,
    }
}

/// 将 rgui LineJoin 转换为 kurbo::Join。
fn to_kurbo_join(join: LineJoin) -> vello::kurbo::Join {
    match join {
        LineJoin::Miter => vello::kurbo::Join::Miter,
        LineJoin::Round => vello::kurbo::Join::Round,
        LineJoin::Bevel => vello::kurbo::Join::Bevel,
    }
}

/// 将 rgui Stroke 转换为 kurbo::Stroke。
fn to_kurbo_stroke(stroke: &Stroke) -> vello::kurbo::Stroke {
    let mut ks = vello::kurbo::Stroke::new(stroke.width.into())
        .with_caps(to_kurbo_cap(stroke.cap))
        .with_join(to_kurbo_join(stroke.join))
        .with_miter_limit(stroke.miter_limit.into());

    if let Some(ref dash) = stroke.dash_pattern {
        let dash_f64: Vec<f64> = dash.iter().map(|&v| v.into()).collect();
        ks = ks.with_dashes(stroke.dash_offset.into(), dash_f64);
    }

    ks
}

/// 将 rgui PathData 转换为 kurbo::BezPath。
fn to_kurbo_path(path_data: &PathData) -> vello::kurbo::BezPath {
    let mut path = vello::kurbo::BezPath::new();
    for cmd in &path_data.commands {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                path.move_to((f64::from(*x), f64::from(*y)));
            },
            PathCommand::LineTo { x, y } => {
                path.line_to((f64::from(*x), f64::from(*y)));
            },
            PathCommand::QuadTo { cx, cy, x, y } => {
                path.quad_to(
                    (f64::from(*cx), f64::from(*cy)),
                    (f64::from(*x), f64::from(*y)),
                );
            },
            PathCommand::CubicTo {
                cx1,
                cy1,
                cx2,
                cy2,
                x,
                y,
            } => {
                path.curve_to(
                    (f64::from(*cx1), f64::from(*cy1)),
                    (f64::from(*cx2), f64::from(*cy2)),
                    (f64::from(*x), f64::from(*y)),
                );
            },
            PathCommand::Close => {
                path.close_path();
            },
        }
    }
    path
}

/// 将 rgui Transform 转换为 kurbo::Affine。
fn to_kurbo_affine(xform: &Transform) -> vello::kurbo::Affine {
    let m = xform.matrix;
    vello::kurbo::Affine::new([
        m[0].into(),
        m[1].into(),
        m[2].into(),
        m[3].into(),
        m[4].into(),
        m[5].into(),
    ])
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{BlendMode, GlyphData};
    use crate::scene::SceneLayer;
    use rgui_core::geometry::Rect;
    use rgui_core::id::WidgetId;

    fn test_params(w: u32, h: u32) -> RenderParams {
        RenderParams {
            width: w,
            height: h,
            ..RenderParams::default()
        }
    }

    #[test]
    fn peniko_color_conversion() {
        let c = Color::new(0.5, 0.25, 0.75, 1.0);
        let pc = to_peniko_color(c);
        // AlphaColor<Srgb> 使用 components: [r, g, b, a]，类型为 f32（0.0-1.0）
        assert!((pc.components[0] - 0.5_f32).abs() < 0.01);
        assert!((pc.components[1] - 0.25_f32).abs() < 0.01);
        assert!((pc.components[2] - 0.75_f32).abs() < 0.01);
        assert!((pc.components[3] - 1.0_f32).abs() < 0.01);
    }

    #[test]
    fn kurbo_rect_conversion() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        let kr = to_kurbo_rect(&r);
        assert_eq!(kr.x0, 10.0);
        assert_eq!(kr.y0, 20.0);
        assert_eq!(kr.x1, 110.0);
        assert_eq!(kr.y1, 70.0);
    }

    #[test]
    fn kurbo_path_conversion() {
        let path_data = PathData {
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
                PathCommand::QuadTo {
                    cx: 15.0,
                    cy: 15.0,
                    x: 20.0,
                    y: 20.0,
                },
                PathCommand::CubicTo {
                    cx1: 25.0,
                    cy1: 25.0,
                    cx2: 30.0,
                    cy2: 30.0,
                    x: 35.0,
                    y: 35.0,
                },
                PathCommand::Close,
            ],
            fill_rule: FillRule::NonZero,
        };
        let kp = to_kurbo_path(&path_data);
        assert!(!kp.elements().is_empty());
    }

    #[test]
    fn encode_empty_scene() {
        let scene = SceneGraph::new(1);
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_fill_rect_scene() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(10.0, 10.0, 50.0, 50.0),
            color: Color::RED,
            radius: 0.0,
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_clip_scene() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::PushClip {
            rect: Rect::new(10.0, 10.0, 50.0, 50.0),
        });
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::RED,
            radius: 0.0,
        });
        layer.push(DrawCommand::PopClip);
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_transform_scene() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::PushTransform {
            transform: Transform::translate(10.0, 20.0),
        });
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 30.0, 30.0),
            color: Color::GREEN,
            radius: 0.0,
        });
        layer.push(DrawCommand::PopTransform);
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_opacity_scene() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::PushOpacity { opacity: 0.5 });
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::RED,
            radius: 0.0,
        });
        layer.push(DrawCommand::PopOpacity);
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 2,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_layer_opacity() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.opacity = 0.75;
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
            color: Color::BLUE,
            radius: 5.0,
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 3,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_rounded_rect() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(10.0, 10.0, 80.0, 80.0),
            color: Color::new(0.2, 0.4, 0.6, 0.8),
            radius: 12.0,
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 4,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_fill_path() {
        let widget_id = WidgetId::new();
        let path = PathData {
            commands: vec![
                PathCommand::MoveTo { x: 50.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 90.0 },
                PathCommand::LineTo { x: 90.0, y: 90.0 },
                PathCommand::Close,
            ],
            fill_rule: FillRule::EvenOdd,
        };
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::FillPath {
            path,
            paint: Paint::Solid(Color::RED),
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 5,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_stroke_path() {
        let widget_id = WidgetId::new();
        let path = PathData {
            commands: vec![
                PathCommand::MoveTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 90.0, y: 90.0 },
            ],
            fill_rule: FillRule::NonZero,
        };
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::StrokePath {
            path,
            stroke: Stroke {
                width: 4.0,
                cap: LineCap::Round,
                join: LineJoin::Round,
                miter_limit: 4.0,
                dash_pattern: Some(vec![8.0, 4.0]),
                dash_offset: 0.0,
            },
            paint: Paint::Solid(Color::GREEN),
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 6,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_gradient_fill() {
        let widget_id = WidgetId::new();
        let path = PathData {
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                PathCommand::LineTo { x: 100.0, y: 0.0 },
                PathCommand::LineTo { x: 100.0, y: 100.0 },
                PathCommand::LineTo { x: 0.0, y: 100.0 },
                PathCommand::Close,
            ],
            fill_rule: FillRule::NonZero,
        };
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::FillPath {
            path,
            paint: Paint::LinearGradient {
                start: crate::primitives::Point::new(0.0, 0.0),
                end: crate::primitives::Point::new(100.0, 100.0),
                stops: vec![
                    crate::primitives::GradientStop {
                        position: 0.0,
                        color: Color::RED,
                    },
                    crate::primitives::GradientStop {
                        position: 1.0,
                        color: Color::BLUE,
                    },
                ],
            },
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 7,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_draw_glyphs() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 200.0, 50.0));
        layer.push(DrawCommand::DrawGlyphs {
            texture_id: TextureId::new(1),
            glyphs: vec![
                GlyphData {
                    atlas_x: 0,
                    atlas_y: 0,
                    atlas_w: 16,
                    atlas_h: 24,
                    offset_x: 10.0,
                    offset_y: 12.0,
                    advance: 14.0,
                },
                GlyphData {
                    atlas_x: 16,
                    atlas_y: 0,
                    atlas_w: 16,
                    atlas_h: 24,
                    offset_x: 24.0,
                    offset_y: 12.0,
                    advance: 14.0,
                },
            ],
            font_size: 20.0,
            color: Color::BLACK,
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 8,
        };
        let params = test_params(200, 50);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_multiple_layers() {
        let id1 = WidgetId::new();
        let id2 = WidgetId::new();

        let mut layer1 = SceneLayer::new(id1, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer1.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::RED,
            radius: 0.0,
        });

        let mut layer2 = SceneLayer::new(id2, 1, Rect::new(25.0, 25.0, 50.0, 50.0));
        layer2.opacity = 0.5;
        layer2.push(DrawCommand::FillRect {
            rect: Rect::new(25.0, 25.0, 50.0, 50.0),
            color: Color::BLUE,
            radius: 0.0,
        });

        let scene = SceneGraph {
            layers: vec![layer1, layer2],
            dirty_layers: vec![0, 1],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 9,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_draw_image_placeholder() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.push(DrawCommand::DrawImage {
            texture_id: TextureId::new(1),
            src: Rect::new(0.0, 0.0, 64.0, 64.0),
            dst: Rect::new(10.0, 10.0, 80.0, 80.0),
            blend_mode: BlendMode::SrcOver,
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 10,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }

    #[test]
    fn encode_skip_zero_opacity_layer() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.opacity = 0.0;
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::RED,
            radius: 0.0,
        });
        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 11,
        };
        let params = test_params(100, 100);
        let vello_scene = encode_scene_to_vello(&scene, &params, None);
        let _ = vello_scene;
    }
}
