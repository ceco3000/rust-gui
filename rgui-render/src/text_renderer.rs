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

    /// 渲染文本为 `DrawCommand::DrawGlyphs`。
    ///
    /// 将文本塑形为字形序列，对每个字形执行光栅化并存入 atlas，
    /// 返回包含所有字形的单条 `DrawGlyphs` 指令。
    ///
    /// `baseline_x`/`baseline_y` 是文本基线在窗口坐标系中的起始位置。
    pub fn render_text(
        &self,
        text: &str,
        baseline_x: f32,
        baseline_y: f32,
        color: Color,
        font_size: f32,
    ) -> Vec<DrawCommand> {
        if text.is_empty() {
            return Vec::new();
        }

        let attrs = cosmic_text::Attrs::new();
        let shaped: Vec<ShapedGlyph> = self.engine.borrow_mut().shape_text(text, font_size, attrs);

        let mut glyphs: Vec<crate::primitives::GlyphData> = Vec::new();
        let texture_id = self.atlas.borrow().texture_id;
        let (atlas_w, atlas_h) = self.atlas.borrow().dimensions();

        for g in &shaped {
            // get_or_rasterize 通过独立 RefCell 访问 engine 和 atlas，
            // 无借用冲突
            let entry = self
                .atlas
                .borrow_mut()
                .get_or_rasterize(g.key.clone(), &mut |key| {
                    self.engine.borrow_mut().rasterize_glyph(key)
                });
            let Some(entry) = entry else {
                continue;
            };

            glyphs.push(crate::primitives::GlyphData {
                atlas_x: (entry.atlas_u * atlas_w as f32) as u32,
                atlas_y: (entry.atlas_v * atlas_h as f32) as u32,
                atlas_w: entry.width as u32,
                atlas_h: entry.height as u32,
                offset_x: baseline_x + g.x,
                offset_y: baseline_y + g.y,
                advance: g.advance,
            });
        }

        if glyphs.is_empty() {
            return Vec::new();
        }

        vec![DrawCommand::DrawGlyphs {
            texture_id,
            glyphs,
            font_size,
            color,
        }]
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
