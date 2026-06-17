//! 文本渲染器——TextEngine + GlyphAtlas 的集成封装。
//!
//! `TextRenderer` 将文本塑形、字形光栅化和 atlas 管理合并为
//! 单一接口，供 `scene_build` 模块在 PaintOp→DrawCommand 转换中使用。
//!
//! 内部使用 `RefCell` 实现共享可变性，允许通过 `&self` 进行
//! 递归遍历中的字形渲染。

use crate::glyph::GlyphAtlas;
use crate::scene::DrawCommand;
use crate::text::{ShapedGlyph, TextEngine};
use crate::texture::TextureId;
use rgui_core::Color;
use std::cell::RefCell;

/// 文本渲染器。
///
/// 持有 `TextEngine`（塑形+光栅化）和 `GlyphAtlas`（字形缓存），
/// 将文本字符串直接转换为 `DrawCommand::DrawGlyphs`。
///
/// 内部使用 `RefCell` 包装可变状态，因此渲染方法接受 `&self`，
/// 可以在递归遍历等需要共享引用的场景中使用。
pub struct TextRenderer {
    engine: RefCell<TextEngine>,
    atlas: RefCell<GlyphAtlas>,
}

// ============================================================================
// TextMetrics
// ============================================================================

/// 文本度量结果。
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    /// 文本总 advance width（像素）。
    pub width: f32,
    /// 基线到字形顶部的距离（像素，正值）。
    pub ascent: f32,
    /// 基线到字形底部的距离（像素，正值，通常为 0 或小值）。
    pub descent: f32,
}

impl TextRenderer {
    /// 创建 TextRenderer。
    ///
    /// `atlas_texture_id` 是预先分配的 wgpu 纹理 ID，atlas 中的字形位图
    /// 将上传到此纹理。
    #[must_use]
    pub fn new(atlas_texture_id: TextureId) -> Self {
        Self {
            engine: RefCell::new(TextEngine::new()),
            atlas: RefCell::new(GlyphAtlas::new(atlas_texture_id, 512, 512)),
        }
    }

    /// 渲染文本，同时返回度量数据（一次光栅化）。
    ///
    /// 将文本塑形为字形序列，对每个字形执行光栅化并存入 atlas，
    /// 返回 `(DrawGlyphs 指令, TextMetrics)`。
    /// TextMetrics 来自光栅化的副产物，不额外消耗。
    ///
    /// `baseline_x`/`baseline_y` 是文本基线在窗口坐标系中的起始位置。
    pub fn render_text(
        &self,
        text: &str,
        baseline_x: f32,
        baseline_y: f32,
        color: Color,
        font_size: f32,
    ) -> (Vec<DrawCommand>, TextMetrics) {
        let empty_metrics = TextMetrics {
            width: 0.0,
            ascent: 0.0,
            descent: 0.0,
        };
        if text.is_empty() {
            return (Vec::new(), empty_metrics);
        }

        let attrs = cosmic_text::Attrs::new();
        let shaped: Vec<ShapedGlyph> = self.engine.borrow_mut().shape_text(text, font_size, attrs);

        let total_width = shaped.last().map(|g| g.x + g.advance).unwrap_or(0.0);

        let mut glyphs: Vec<crate::primitives::GlyphData> = Vec::new();
        let texture_id = self.atlas.borrow().texture_id;
        let (atlas_w, atlas_h) = self.atlas.borrow().dimensions();

        let mut max_ascent: f32 = 0.0;
        let mut max_descent: f32 = 0.0;

        for g in &shaped {
            let entry = self
                .atlas
                .borrow_mut()
                .get_or_rasterize(g.key.clone(), &mut |key| {
                    self.engine.borrow_mut().rasterize_glyph(key)
                });
            let Some(entry) = entry else {
                continue;
            };

            // 从 atlas entry 读取 placement 副产物
            let ascent = entry.top as f32;
            max_ascent = max_ascent.max(ascent);
            let descent = (entry.height as i32 - entry.top).max(0) as f32;
            max_descent = max_descent.max(descent);

            glyphs.push(crate::primitives::GlyphData {
                atlas_x: (entry.atlas_u * atlas_w as f32) as u32,
                atlas_y: (entry.atlas_v * atlas_h as f32) as u32,
                atlas_w: entry.width as u32,
                atlas_h: entry.height as u32,
                offset_x: baseline_x + g.x,
                offset_y: baseline_y + g.y,
                advance: g.advance,
                glyph_index: g.key.glyph_id,
            });
        }

        if glyphs.is_empty() {
            return (Vec::new(), empty_metrics);
        }

        (
            vec![DrawCommand::DrawGlyphs {
                texture_id,
                glyphs,
                font_size,
                color,
            }],
            TextMetrics {
                width: total_width,
                ascent: max_ascent,
                descent: max_descent,
            },
        )
    }

    /// 测量文本宽度（像素单位）。
    ///
    /// 对文本进行塑形，返回字形序列的总 advance width。
    /// 不执行光栅化或 atlas 分配，仅用于布局计算。
    #[must_use]
    pub fn measure_text(&self, text: &str, font_size: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let attrs = cosmic_text::Attrs::new();
        let shaped = self.engine.borrow_mut().shape_text(text, font_size, attrs);
        shaped.last().map(|g| g.x + g.advance).unwrap_or(0.0)
    }

    /// 测量文本的精确度量（宽度、ascent、descent）。
    ///
    /// 对每个字形执行光栅化以获取实际 placement 数据，
    /// 返回可用于精确居中的度量信息。
    /// `ascent` = 基线到字形顶部的实际像素距离（正值）。
    /// `descent` = 基线到字形底部的实际像素距离（正值）。
    #[must_use]
    pub fn measure_text_metrics(&self, text: &str, font_size: f32) -> TextMetrics {
        if text.is_empty() {
            return TextMetrics {
                width: 0.0,
                ascent: 0.0,
                descent: 0.0,
            };
        }
        let attrs = cosmic_text::Attrs::new();
        let shaped = self.engine.borrow_mut().shape_text(text, font_size, attrs);

        let total_width = shaped.last().map(|g| g.x + g.advance).unwrap_or(0.0);

        let mut max_ascent: f32 = 0.0;
        let mut max_descent: f32 = 0.0;

        for g in &shaped {
            if let Some((_left, top, _w, h)) = self.engine.borrow_mut().glyph_placement(&g.key) {
                // zeno::Placement: origin 在图像内坐标 (left, top)，
                // top = 基线到图像顶部的像素距离（正值 = 基线上方）
                // height - top = 基线下方像素数（若为正值）
                let ascent = top as f32;
                max_ascent = max_ascent.max(ascent);
                let descent = (h as i32 - top).max(0) as f32;
                max_descent = max_descent.max(descent);
            }
        }

        TextMetrics {
            width: total_width,
            ascent: max_ascent,
            descent: max_descent,
        }
    }

    /// 返回 atlas 中待上传到 GPU 的脏区域列表。
    #[must_use]
    pub fn pending_uploads(&self) -> Vec<crate::glyph::UploadRect> {
        self.atlas.borrow().upload_queue.clone()
    }

    /// 标记 atlas 为干净（上传完成后调用）。
    pub fn clear_dirty(&self) {
        let mut atlas = self.atlas.borrow_mut();
        atlas.dirty = false;
        atlas.upload_queue.clear();
    }

    /// atlas 纹理是否有新的脏数据需要上传。
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.atlas.borrow().dirty
    }

    /// 返回 atlas 纹理 ID。
    #[must_use]
    pub fn atlas_texture_id(&self) -> TextureId {
        self.atlas.borrow().texture_id
    }

    /// 返回 atlas 纹理尺寸（宽, 高）。
    #[must_use]
    pub fn atlas_dimensions(&self) -> (u32, u32) {
        self.atlas.borrow().dimensions()
    }

    /// 返回 CPU-side atlas 像素缓冲区引用（RGBA8, w × h × 4 bytes）。
    #[must_use]
    pub fn atlas_pixels(&self) -> Vec<u8> {
        self.atlas.borrow().pixels().to_vec()
    }
}

impl std::fmt::Debug for TextRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let atlas = self.atlas.borrow();
        f.debug_struct("TextRenderer")
            .field("atlas_texture_id", &atlas.texture_id)
            .field("dirty", &atlas.dirty)
            .finish()
    }
}
