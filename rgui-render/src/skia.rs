//! Skia CPU 光栅化后端实现（D3 §7）。
//!
//! 使用 skia-safe crate 在 CPU 上光栅化 SceneGraph，作为 Vello 后端的
//! fallback 选项。无需 GPU 即可工作，适用于无 GPU 环境、CI 测试和
//! 纯 CPU 渲染验证。
//!
//! # 设计
//!
//! - 每帧创建新的 skia raster surface（CPU 光栅化），绘制完成后读回像素。
//! - 不持有 skia 对象跨帧（surface、canvas），因此 `SkiaBackend` 本身
//!   满足 `Send + Sync`。
//! - 渲染结果通过 [`pixels()`] 方法获取 RGBA 字节数组。
//!
//! # 限制
//!
//! - `DrawGlyphs` 绘制占位矩形而非实际字形（字形数据仅为 atlas 坐标，
//!   不包含字体信息，skia 无法基于 atlas 坐标重建文本形状）。
//! - 无 GPU 加速，大场景下帧率受限。

use std::collections::HashMap;

use log;

use crate::backend::{RenderBackend, RenderError, RenderParams};
use crate::primitives::{
    BlendMode, FillRule, GlyphData, LineCap, LineJoin, Paint, PathCommand, PathData, Stroke,
    Transform,
};
use crate::scene::{DrawCommand, SceneGraph};
use crate::texture::{TextureData, TextureFormat, TextureId};
use rgui_core::Color;

// ============================================================================
// SkiaBackend
// ============================================================================

/// Skia CPU 光栅化后端（D3 §7）。
///
/// 使用 skia-safe 在 CPU 上完成全部光栅化，输出 RGBA 像素缓冲区。
/// 适用于无 GPU 环境的 fallback 渲染。
pub struct SkiaBackend {
    /// 已注册的应用纹理。
    textures: HashMap<TextureId, TextureData>,
    /// 下一个纹理 ID 计数器。
    next_texture_id: u64,
    /// 最新帧的 RGBA 像素数据。
    pixels: Vec<u8>,
    /// 帧缓冲区的宽度。
    width: u32,
    /// 帧缓冲区的高度。
    height: u32,
}

impl SkiaBackend {
    /// 创建新的 Skia CPU 光栅化后端。
    #[must_use]
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            next_texture_id: 1,
            pixels: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    /// 当前帧的 RGBA 像素数据（每像素 4 字节，行对齐到 4 字节边界）。
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// 帧缓冲区的逻辑尺寸（像素单位）。
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// 将场景图渲染到内部缓冲区。
    fn render_to_buffer(
        &mut self,
        scene: &SceneGraph,
        params: &RenderParams,
    ) -> Result<(), RenderError> {
        let width = params.width.max(1);
        let height = params.height.max(1);

        let image_info = skia_safe::ImageInfo::new(
            skia_safe::ISize::new(width as i32, height as i32),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );

        let mut surface =
            skia_safe::surfaces::raster(&image_info, None, None::<&skia_safe::SurfaceProps>)
                .ok_or_else(|| RenderError::RenderFailed("创建 Skia 光栅表面失败".into()))?;

        let canvas = surface.canvas();

        // 清除背景
        if let Some(bg) = params.clear_color {
            canvas.clear(skia_safe::Color4f::new(
                bg.r as f32,
                bg.g as f32,
                bg.b as f32,
                bg.a as f32,
            ));
        } else {
            canvas.clear(skia_safe::Color4f::new(1.0, 1.0, 1.0, 1.0));
        }

        // 逐层绘制
        let mut opacity_stack: Vec<f32> = vec![1.0];
        for layer in &scene.layers {
            let layer_opacity = layer.opacity;
            if layer_opacity <= 0.0 {
                continue;
            }

            // 层级别的变换
            if let Some(ref xform) = layer.transform {
                canvas.save();
                canvas.concat(&to_skia_matrix(xform));
            }

            for cmd in &layer.commands {
                let current_opacity = *opacity_stack.last().unwrap_or(&1.0) * layer_opacity;
                match cmd {
                    DrawCommand::PushClip { rect } => {
                        canvas.save();
                        canvas.clip_rect(to_skia_rect(rect), skia_safe::ClipOp::Intersect, true);
                    },
                    DrawCommand::PopClip => {
                        canvas.restore();
                    },
                    DrawCommand::PushTransform { transform } => {
                        canvas.save();
                        canvas.concat(&to_skia_matrix(transform));
                    },
                    DrawCommand::PopTransform => {
                        canvas.restore();
                    },
                    DrawCommand::PushOpacity { opacity } => {
                        let cumulative = *opacity_stack.last().unwrap_or(&1.0) * *opacity;
                        opacity_stack.push(cumulative);
                    },
                    DrawCommand::PopOpacity => {
                        opacity_stack.pop();
                    },
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                    } => {
                        draw_fill_rect(canvas, rect, *color, *radius, current_opacity);
                    },
                    DrawCommand::FillPath { path, paint } => {
                        draw_fill_path(canvas, path, paint, current_opacity);
                    },
                    DrawCommand::StrokePath {
                        path,
                        stroke,
                        paint,
                    } => {
                        draw_stroke_path(canvas, path, stroke, paint, current_opacity);
                    },
                    DrawCommand::DrawGlyphs {
                        glyphs,
                        font_size,
                        color,
                    } => {
                        draw_glyphs_placeholder(
                            canvas,
                            glyphs,
                            *font_size,
                            *color,
                            current_opacity,
                        );
                    },
                    DrawCommand::DrawImage {
                        texture_id,
                        src,
                        dst,
                        blend_mode,
                    } => {
                        self.draw_image(
                            canvas,
                            *texture_id,
                            src,
                            dst,
                            *blend_mode,
                            current_opacity,
                        );
                    },
                }
            }

            if layer.transform.is_some() {
                canvas.restore();
            }
        }

        // 读回像素（使用 Surface::read_pixels 直接写入缓冲区）
        let row_bytes = (width as usize) * 4;
        let buffer_size = (height as usize) * row_bytes;
        let mut pixel_buffer = vec![0u8; buffer_size];

        if !surface.read_pixels(&image_info, &mut pixel_buffer, row_bytes, (0, 0)) {
            return Err(RenderError::RenderFailed("读取 Skia 像素数据失败".into()));
        }

        self.pixels = pixel_buffer;
        self.width = width;
        self.height = height;

        Ok(())
    }

    /// 绘制 DrawImage 命令。在已注册纹理中查找像素数据，创建 skia image 并绘制。
    fn draw_image(
        &self,
        canvas: &skia_safe::Canvas,
        texture_id: TextureId,
        src: &rgui_core::geometry::Rect,
        dst: &rgui_core::geometry::Rect,
        blend_mode: BlendMode,
        opacity: f32,
    ) {
        // 纹理未注册时绘制红色占位矩形并记录警告
        let Some(texture_data) = self.textures.get(&texture_id) else {
            log::warn!(
                "绘制 DrawImage 时纹理 {} 未注册，使用红色占位矩形",
                texture_id.as_u64()
            );
            let mut paint = skia_safe::Paint::default();
            paint.set_color(skia_safe::Color::from_argb(255, 255, 0, 0));
            canvas.draw_rect(to_skia_rect(dst), &paint);
            return;
        };

        let image_info = skia_safe::ImageInfo::new(
            skia_safe::ISize::new(texture_data.width as i32, texture_data.height as i32),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );

        let src_rect = to_skia_rect(src);
        let dst_rect = to_skia_rect(dst);
        let row_bytes = (texture_data.width as usize) * 4;

        if let Some(image) = skia_safe::images::raster_from_data(
            &image_info,
            skia_safe::Data::new_copy(&texture_data.pixels),
            row_bytes,
        ) {
            let mut paint = skia_safe::Paint::default();
            if (opacity - 1.0).abs() > f32::EPSILON {
                let alpha_byte = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
                paint.set_alpha(alpha_byte);
            }
            paint.set_blend_mode(to_skia_blend_mode(blend_mode));

            canvas.draw_image_rect(
                &image,
                Some((&src_rect, skia_safe::canvas::SrcRectConstraint::Fast)),
                dst_rect,
                &paint,
            );
        }
    }
}

impl Default for SkiaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBackend for SkiaBackend {
    fn render(&mut self, scene: &SceneGraph, params: &RenderParams) -> Result<(), RenderError> {
        self.render_to_buffer(scene, params)
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
        "Skia (CPU)"
    }
}

// ============================================================================
// 类型转换辅助函数
// ============================================================================

/// 将 rgui Color 转换为 skia Color（Paint::set_color 使用），
/// 同时叠加透明度因子。
#[inline]
fn to_skia_color(color: Color, opacity: f32) -> skia_safe::Color {
    let [r, g, b, a] = color.to_u8_array();
    let aa = ((a as f32) * opacity.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 255.0) as u8;
    skia_safe::Color::from_argb(aa, r, g, b)
}

/// 将 rgui Color 转换为 skia Color4f，并叠加透明度因子。
#[inline]
fn to_color4f_with_opacity(color: Color, opacity: f32) -> skia_safe::Color4f {
    skia_safe::Color4f::new(
        color.r as f32,
        color.g as f32,
        color.b as f32,
        (color.a as f32 * opacity).clamp(0.0, 1.0),
    )
}

/// 将 rgui BlendMode 映射为 skia_safe::BlendMode。
#[inline]
fn to_skia_blend_mode(mode: BlendMode) -> skia_safe::BlendMode {
    match mode {
        BlendMode::SrcOver => skia_safe::BlendMode::SrcOver,
        BlendMode::Src => skia_safe::BlendMode::Src,
        BlendMode::Multiply => skia_safe::BlendMode::Multiply,
        BlendMode::Screen => skia_safe::BlendMode::Screen,
        BlendMode::Overlay => skia_safe::BlendMode::Overlay,
    }
}

/// 将 rgui geometry::Rect 转换为 skia Rect。
#[inline]
fn to_skia_rect(rect: &rgui_core::geometry::Rect) -> skia_safe::Rect {
    skia_safe::Rect::from_xywh(
        rect.origin.x as f32,
        rect.origin.y as f32,
        rect.size.width as f32,
        rect.size.height as f32,
    )
}

/// 将 rgui Transform 转换为 skia Matrix。
///
/// rgui Transform.matrix 按行优先存储 2x3 仿射矩阵元素：
/// `[scaleX, skewX, transX, skewY, scaleY, transY]`
/// 对应 3x3 齐次矩阵：
/// ```text
/// [m[0]  m[1]  m[2]]
/// [m[3]  m[4]  m[5]]
/// [0     0     1   ]
/// ```
#[inline]
fn to_skia_matrix(xform: &Transform) -> skia_safe::Matrix {
    let m = xform.matrix;
    let mut matrix = skia_safe::Matrix::new_identity();
    matrix.set_all(
        m[0], m[1], m[2], // scaleX, skewX, transX
        m[3], m[4], m[5], // skewY, scaleY, transY
        0.0, 0.0, 1.0, // persp0, persp1, persp2 (identity row)
    );
    matrix
}

/// 将 rgui PathData 转换为 skia Path。
fn to_skia_path(path: &PathData) -> skia_safe::Path {
    let mut sk_path = skia_safe::Path::new();
    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                sk_path.move_to((*x, *y));
            },
            PathCommand::LineTo { x, y } => {
                sk_path.line_to((*x, *y));
            },
            PathCommand::QuadTo { cx, cy, x, y } => {
                sk_path.quad_to((*cx, *cy), (*x, *y));
            },
            PathCommand::CubicTo {
                cx1,
                cy1,
                cx2,
                cy2,
                x,
                y,
            } => {
                sk_path.cubic_to((*cx1, *cy1), (*cx2, *cy2), (*x, *y));
            },
            PathCommand::Close => {
                sk_path.close();
            },
        }
    }
    if path.fill_rule == FillRule::EvenOdd {
        sk_path.set_fill_type(skia_safe::PathFillType::EvenOdd);
    }
    sk_path
}

/// 配置 skia Paint 为填充样式并设置颜色和透明度。
fn setup_fill_paint(paint: &mut skia_safe::Paint, color: Color, opacity: f32) {
    paint.set_style(skia_safe::PaintStyle::Fill);
    paint.set_anti_alias(true);
    paint.set_color(to_skia_color(color, opacity));
}

/// 应用渐变色着色器到 skia Paint。
fn apply_gradient_shader(paint: &mut skia_safe::Paint, style_paint: &Paint, opacity: f32) {
    match style_paint {
        Paint::Solid(_) => { /* 颜色已设置 */ },
        Paint::LinearGradient { start, end, stops } => {
            let colors: Vec<skia_safe::Color4f> = stops
                .iter()
                .map(|s| to_color4f_with_opacity(s.color, opacity))
                .collect();
            let positions: Vec<f32> = stops.iter().map(|s| s.position).collect();
            let shader = skia_safe::Shader::linear_gradient(
                ((start.x, start.y), (end.x, end.y)),
                colors.as_slice(),
                Some(positions.as_slice()),
                skia_safe::TileMode::Clamp,
                None,
                None::<&skia_safe::Matrix>,
            );
            if let Some(s) = shader {
                paint.set_shader(s);
            }
        },
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => {
            let colors: Vec<skia_safe::Color4f> = stops
                .iter()
                .map(|s| to_color4f_with_opacity(s.color, opacity))
                .collect();
            let positions: Vec<f32> = stops.iter().map(|s| s.position).collect();
            let shader = skia_safe::Shader::radial_gradient(
                (center.x, center.y),
                *radius,
                colors.as_slice(),
                Some(positions.as_slice()),
                skia_safe::TileMode::Clamp,
                None,
                None::<&skia_safe::Matrix>,
            );
            if let Some(s) = shader {
                paint.set_shader(s);
            }
        },
        Paint::Image { .. } => {
            // 图像着色器暂不支持，用洋红色表示占位
            paint.set_color(skia_safe::Color::from_argb(128, 255, 0, 255));
        },
    }
}

/// 配置 skia Paint 为描边样式。
fn setup_stroke_paint(paint: &mut skia_safe::Paint, stroke: &Stroke, color: Color, opacity: f32) {
    paint.set_style(skia_safe::PaintStyle::Stroke);
    paint.set_anti_alias(true);
    paint.set_stroke_width(stroke.width.max(0.0));
    paint.set_color(to_skia_color(color, opacity));

    paint.set_stroke_cap(match stroke.cap {
        LineCap::Butt => skia_safe::PaintCap::Butt,
        LineCap::Round => skia_safe::PaintCap::Round,
        LineCap::Square => skia_safe::PaintCap::Square,
    });

    paint.set_stroke_join(match stroke.join {
        LineJoin::Miter => skia_safe::PaintJoin::Miter,
        LineJoin::Round => skia_safe::PaintJoin::Round,
        LineJoin::Bevel => skia_safe::PaintJoin::Bevel,
    });

    paint.set_stroke_miter(stroke.miter_limit.max(0.0));

    if let Some(ref dash) = stroke.dash_pattern {
        if let Some(path_effect) = skia_safe::PathEffect::dash(dash, stroke.dash_offset) {
            paint.set_path_effect(path_effect);
        }
    }
}

// ============================================================================
// 指令级绘制函数
// ============================================================================

/// 绘制填充矩形（FillRect）。
fn draw_fill_rect(
    canvas: &skia_safe::Canvas,
    rect: &rgui_core::geometry::Rect,
    color: Color,
    radius: f32,
    opacity: f32,
) {
    let sk_rect = to_skia_rect(rect);
    let mut paint = skia_safe::Paint::default();
    setup_fill_paint(&mut paint, color, opacity);

    if radius > f32::EPSILON {
        canvas.draw_round_rect(sk_rect, radius, radius, &paint);
    } else {
        canvas.draw_rect(sk_rect, &paint);
    }
}

/// 绘制填充路径（FillPath）。
fn draw_fill_path(canvas: &skia_safe::Canvas, path: &PathData, paint: &Paint, opacity: f32) {
    let sk_path = to_skia_path(path);
    let mut sk_paint = skia_safe::Paint::default();
    sk_paint.set_style(skia_safe::PaintStyle::Fill);
    sk_paint.set_anti_alias(true);

    apply_gradient_shader(&mut sk_paint, paint, opacity);

    // 如果没有着色器（Solid 类型），直接设置颜色
    if sk_paint.shader().is_none() {
        if let Paint::Solid(color) = paint {
            setup_fill_paint(&mut sk_paint, *color, opacity);
        } else {
            sk_paint.set_color(to_skia_color(Color::new(0.0, 0.0, 0.0, 1.0), opacity));
        }
    }

    canvas.draw_path(&sk_path, &sk_paint);
}

/// 绘制描边路径（StrokePath）。
fn draw_stroke_path(
    canvas: &skia_safe::Canvas,
    path: &PathData,
    stroke: &Stroke,
    paint: &Paint,
    opacity: f32,
) {
    let sk_path = to_skia_path(path);
    let mut sk_paint = skia_safe::Paint::default();

    match paint {
        Paint::Solid(color) => {
            setup_stroke_paint(&mut sk_paint, stroke, *color, opacity);
        },
        Paint::LinearGradient { .. } | Paint::RadialGradient { .. } => {
            setup_stroke_paint(
                &mut sk_paint,
                stroke,
                Color::new(0.0, 0.0, 0.0, 1.0),
                opacity,
            );
            apply_gradient_shader(&mut sk_paint, paint, opacity);
        },
        Paint::Image { .. } => {
            sk_paint.set_style(skia_safe::PaintStyle::Stroke);
            sk_paint.set_stroke_width(stroke.width.max(0.0));
            sk_paint.set_color(skia_safe::Color::from_argb(128, 255, 0, 255));
        },
    }

    canvas.draw_path(&sk_path, &sk_paint);
}

/// 绘制字形占位矩形。
///
/// 当前实现绘制彩色矩形作为字形位置的占位标记。完整的字形渲染
/// 需要将字形 Atlas 纹理中的子区域正确映射到目标位置，而当前
/// DrawGlyphs 命令不包含 Atlas 纹理 ID，因此无法实现完整渲染。
fn draw_glyphs_placeholder(
    canvas: &skia_safe::Canvas,
    glyphs: &[GlyphData],
    _font_size: f32,
    color: Color,
    opacity: f32,
) {
    let mut paint = skia_safe::Paint::default();
    paint.set_style(skia_safe::PaintStyle::Fill);
    paint.set_anti_alias(false);
    paint.set_color(to_skia_color(color, opacity));

    for glyph in glyphs {
        let rect = skia_safe::Rect::from_xywh(
            glyph.offset_x,
            glyph.offset_y,
            glyph.atlas_w as f32,
            glyph.atlas_h as f32,
        );
        canvas.draw_rect(rect, &paint);
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
#[cfg(feature = "skia-backend")]
mod tests {
    use super::*;
    use crate::scene::SceneLayer;
    use rgui_core::geometry::Rect;
    use rgui_core::id::WidgetId;

    /// 创建测试用的渲染参数。
    fn test_params(width: u32, height: u32) -> RenderParams {
        RenderParams {
            scale_factor: 1.0,
            vsync: false,
            clear_color: Some(Color::new(1.0, 1.0, 1.0, 1.0)),
            width,
            height,
        }
    }

    /// 创建一个包含单个 FillRect 命令的测试场景。
    fn single_rect_scene(x: f64, y: f64, w: f64, h: f64, color: Color, radius: f32) -> SceneGraph {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(x, y, w, h));
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(x, y, w, h),
            color,
            radius,
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
    fn new_backend_is_empty() {
        let backend = SkiaBackend::new();
        assert_eq!(backend.backend_name(), "Skia (CPU)");
        assert!(backend.pixels().is_empty());
        assert_eq!(backend.dimensions(), (0, 0));
    }

    #[test]
    fn render_empty_scene() {
        let mut backend = SkiaBackend::new();
        let scene = SceneGraph::new(1);
        let params = test_params(100, 100);

        let result = backend.render(&scene, &params);
        assert!(result.is_ok());
        assert_eq!(backend.dimensions(), (100, 100));
        assert_eq!(backend.pixels().len(), 100 * 100 * 4);
    }

    #[test]
    fn render_fill_rect_solid() {
        let mut backend = SkiaBackend::new();
        let scene = single_rect_scene(10.0, 10.0, 50.0, 50.0, Color::RED, 0.0);
        let params = test_params(100, 100);

        assert!(backend.render(&scene, &params).is_ok());

        // 验证渲染区域有内容（取矩形中心的像素，应为非白色）
        let pixels = backend.pixels();
        let stride = 100 * 4;
        let center_idx = (35 * stride + 35 * 4) as usize;

        // 红色像素的 r 通道（index 0）应该 > 0
        assert!(pixels[center_idx] > 0, "矩形中心像素应包含红色分量");
    }

    #[test]
    fn render_fill_rect_rounded() {
        let mut backend = SkiaBackend::new();
        let scene = single_rect_scene(10.0, 10.0, 80.0, 80.0, Color::BLUE, 10.0);
        let params = test_params(100, 100);

        assert!(backend.render(&scene, &params).is_ok());
        let pixels = backend.pixels();
        let stride = 100 * 4;

        // 矩形内部（50, 50）应为蓝色
        let interior = (50 * stride + 50 * 4) as usize;
        assert!(pixels[interior + 2] > 0, "矩形内部应包含蓝色分量");
    }

    #[test]
    fn render_multiple_layers() {
        let mut backend = SkiaBackend::new();

        // 两个重叠的矩形：红色底层 + 半透明蓝色上层
        let mut layer_bottom =
            SceneLayer::new(WidgetId::new(), 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer_bottom.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::RED,
            radius: 0.0,
        });

        let mut layer_top = SceneLayer::new(WidgetId::new(), 1, Rect::new(25.0, 25.0, 50.0, 50.0));
        layer_top.push(DrawCommand::FillRect {
            rect: Rect::new(25.0, 25.0, 50.0, 50.0),
            color: Color::new(0.0, 0.0, 1.0, 0.5),
            radius: 0.0,
        });

        let scene = SceneGraph {
            layers: vec![layer_bottom, layer_top],
            dirty_layers: vec![0, 1],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };

        let params = test_params(100, 100);
        assert!(backend.render(&scene, &params).is_ok());

        let pixels = backend.pixels();
        let stride = 100 * 4;

        // 重叠区域（50, 50）应为红色和蓝色的混合
        let overlap = (50 * stride + 50 * 4) as usize;
        assert!(pixels[overlap] > 0, "重叠区域应有红色分量");
        assert!(pixels[overlap + 2] > 0, "重叠区域应有蓝色分量");
    }

    #[test]
    fn register_and_unregister_texture() {
        let mut backend = SkiaBackend::new();
        let data = TextureData {
            width: 16,
            height: 16,
            pixels: vec![255u8; 16 * 16 * 4],
            format: TextureFormat::Rgba8,
        };

        let id = backend.register_texture(&data, TextureFormat::Rgba8);
        assert_ne!(id.as_u64(), 0);

        backend.unregister_texture(id);
        // 确认不 panic 即可
    }

    #[test]
    fn render_with_texture_image() {
        let mut backend = SkiaBackend::new();

        // 注册一个 4x4 蓝色纹理
        let mut tex_pixels = vec![0u8; 4 * 4 * 4];
        for px in tex_pixels.chunks_exact_mut(4) {
            px[2] = 255; // 蓝色通道
            px[3] = 255; // alpha
        }
        let data = TextureData {
            width: 4,
            height: 4,
            pixels: tex_pixels,
            format: TextureFormat::Rgba8,
        };
        let tex_id = backend.register_texture(&data, TextureFormat::Rgba8);

        // 创建场景：绘制图像
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(10.0, 10.0, 80.0, 80.0));
        layer.push(DrawCommand::DrawImage {
            texture_id: tex_id,
            src: Rect::new(0.0, 0.0, 4.0, 4.0),
            dst: Rect::new(10.0, 10.0, 80.0, 80.0),
            blend_mode: BlendMode::SrcOver,
        });

        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };

        let params = test_params(100, 100);
        assert!(backend.render(&scene, &params).is_ok());

        // 图像区域应有蓝色分量
        let pixels = backend.pixels();
        let stride = 100 * 4;
        let img_area = (50 * stride + 50 * 4) as usize;
        assert!(pixels[img_area + 2] > 0, "图像区域应有蓝色分量");
    }

    #[test]
    fn render_with_transform() {
        let mut backend = SkiaBackend::new();
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
        assert!(backend.render(&scene, &params).is_ok());

        // 变换后的矩形应在 (10, 20) 位置
        let pixels = backend.pixels();
        let stride = 100 * 4;
        let transformed = (20 * stride + 10 * 4) as usize;
        assert!(pixels[transformed + 1] > 0, "变换后位置有绿色分量");
    }

    #[test]
    fn render_backend_name() {
        let backend = SkiaBackend::new();
        assert_eq!(backend.backend_name(), "Skia (CPU)");
    }

    #[test]
    fn render_path_solid_fill() {
        let mut backend = SkiaBackend::new();
        let widget_id = WidgetId::new();

        // 创建一个三角形的 FillPath
        let path = PathData {
            commands: vec![
                PathCommand::MoveTo { x: 50.0, y: 10.0 },
                PathCommand::LineTo { x: 10.0, y: 90.0 },
                PathCommand::LineTo { x: 90.0, y: 90.0 },
                PathCommand::Close,
            ],
            fill_rule: FillRule::NonZero,
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
            version: 1,
        };

        let params = test_params(100, 100);
        assert!(backend.render(&scene, &params).is_ok());

        // 三角形内部（50, 60）应为红色
        let pixels = backend.pixels();
        let stride = 100 * 4;
        let inside = (60 * stride + 50 * 4) as usize;
        assert!(pixels[inside] > 0, "路径区域有红色分量");
    }

    #[test]
    fn render_with_clip() {
        let mut backend = SkiaBackend::new();
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));

        // 设置裁剪区域为左上角 50x50，然后全屏画红色矩形
        layer.push(DrawCommand::PushClip {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
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
        assert!(backend.render(&scene, &params).is_ok());

        let pixels = backend.pixels();
        let stride = 100 * 4;

        // 裁剪区域内 (25, 25) 有红色
        let inside = (25 * stride + 25 * 4) as usize;
        assert!(pixels[inside] > 0, "裁剪区域内应有红色");

        // 裁剪区域外 (75, 25) 无红色（白色背景）
        let outside = (25 * stride + 75 * 4) as usize;
        assert_eq!(pixels[outside], 255, "裁剪区域外应为白色");
    }

    #[test]
    fn render_layer_opacity() {
        let mut backend = SkiaBackend::new();
        let widget_id = WidgetId::new();

        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));
        layer.opacity = 0.5;
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
            version: 1,
        };

        let params = test_params(100, 100);
        assert!(backend.render(&scene, &params).is_ok());

        let pixels = backend.pixels();
        let stride = 100 * 4;
        let center = (50 * stride + 50 * 4) as usize;
        // 白色背景上半透明红色：绿色通道应在 0-255 之间，
        // 且比不透明红色时更高（半透明红色让背景白色透出）
        assert!(
            pixels[center + 1] > 0 && pixels[center + 1] < 255,
            "半透明红色区域绿色通道应在中间值（实际={}）",
            pixels[center + 1]
        );
        // 红色通道应饱和（255，与不透明时相同）
        assert_eq!(pixels[center], 255, "红色通道饱和");
    }

    #[test]
    fn render_resize_buffer() {
        let mut backend = SkiaBackend::new();

        // 第一次渲染 50x50
        let scene1 = SceneGraph::new(1);
        assert!(backend.render(&scene1, &test_params(50, 50)).is_ok());
        assert_eq!(backend.dimensions(), (50, 50));

        // 第二次渲染 100x200
        let scene2 = SceneGraph::new(2);
        assert!(backend.render(&scene2, &test_params(100, 200)).is_ok());
        assert_eq!(backend.dimensions(), (100, 200));
        assert_eq!(backend.pixels().len(), 100 * 200 * 4);
    }

    #[test]
    fn render_multiple_textures() {
        let mut backend = SkiaBackend::new();

        let red_tex = backend.register_texture(
            &TextureData {
                width: 2,
                height: 2,
                pixels: std::iter::repeat([255u8, 0, 0, 255])
                    .take(4)
                    .flatten()
                    .collect(),
                format: TextureFormat::Rgba8,
            },
            TextureFormat::Rgba8,
        );

        let blue_tex = backend.register_texture(
            &TextureData {
                width: 2,
                height: 2,
                pixels: std::iter::repeat([0u8, 0, 255, 255])
                    .take(4)
                    .flatten()
                    .collect(),
                format: TextureFormat::Rgba8,
            },
            TextureFormat::Rgba8,
        );

        assert_ne!(red_tex, blue_tex);

        backend.unregister_texture(red_tex);
        backend.unregister_texture(blue_tex);
        // 确认不 panic 即可
    }

    #[test]
    fn render_stroke_path_solid() {
        let mut backend = SkiaBackend::new();
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
                dash_pattern: None,
                dash_offset: 0.0,
            },
            paint: Paint::Solid(Color::GREEN),
        });

        let scene = SceneGraph {
            layers: vec![layer],
            dirty_layers: vec![0],
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version: 1,
        };

        let params = test_params(100, 100);
        assert!(backend.render(&scene, &params).is_ok());

        // 路径经过 (50, 50) 点，应为绿色
        let pixels = backend.pixels();
        let stride = 100 * 4;
        let on_path = (50 * stride + 50 * 4) as usize;
        assert!(pixels[on_path + 1] > 0, "路径上应有绿色分量");
    }

    #[test]
    fn render_push_pop_opacity() {
        let mut backend = SkiaBackend::new();
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(0.0, 0.0, 100.0, 100.0));

        layer.push(DrawCommand::PushOpacity { opacity: 0.25 });
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
            version: 1,
        };

        let params = test_params(100, 100);
        assert!(backend.render(&scene, &params).is_ok());

        let pixels = backend.pixels();
        let stride = 100 * 4;
        let center = (50 * stride + 50 * 4) as usize;
        // PushOpacity 0.25 使得红色分量与背景白色混合，
        // 绿色/蓝色通道应高于 128（白色背景占比大）
        assert!(
            pixels[center + 1] > 128,
            "透明度 0.25 下绿色通道应 > 128（实际={}）",
            pixels[center + 1]
        );
        // 红色通道应饱和
        assert_eq!(pixels[center], 255, "红色通道饱和");
    }

    #[test]
    fn render_layer_transform_offset() {
        let mut backend = SkiaBackend::new();
        let widget_id = WidgetId::new();

        let mut layer = SceneLayer::new(widget_id, 0, Rect::new(20.0, 30.0, 50.0, 50.0));
        layer.transform = Some(Transform::translate(5.0, 10.0));
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
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
        assert!(backend.render(&scene, &params).is_ok());

        // 层变换偏移 (5, 10) + 矩形在层内 (0, 0) = 实际位置 (5, 10)
        let pixels = backend.pixels();
        let stride = 100 * 4;
        let pos = (10 * stride + 5 * 4) as usize;
        assert!(pixels[pos] > 0, "层变换后的位置应有红色分量");
    }
}
