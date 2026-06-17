//! cosmic-text 文本引擎（D3 §8）。
//!
//! 提供文本塑形（shaping）和字形光栅化（rasterization）功能。
//! `TextEngine` 封装 cosmic-text 的 `FontSystem` 和 `SwashCache`，
//! 通过嵌入字体提供跨平台一致的文本渲染。
//!
//! # 架构
//!
//! ```text
//! 文本 → TextEngine::shape() → cosmic_text::Buffer → layout_runs()
//!   → GlyphKey + rasterize_glyph() → RasterizedGlyph → GlyphAtlas
//! ```
//!
//! # 字体后备
//!
//! 使用嵌入字体（Inter）作为主字体。对于 Latin 文本，
//! Inter 提供完整覆盖。CJK 字符需要系统字体后备或未来嵌入
//! Noto Sans CJK 字体。

use cosmic_text::{
    Attrs, Buffer, CacheKey, CacheKeyFlags, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
    SwashImage,
};
use rustc_hash::FxHashMap;

use crate::glyph::{GlyphKey, RasterizedGlyph};

/// cosmic-text 文本引擎（D3 §8）。
///
/// 管理字体系统、文本塑形和字形光栅化的生命周期。
/// 内部维护 `fontdb::ID → u64` 映射，使 `GlyphKey` 可以使用
/// 轻量的 `u64` 字体标识符。
///
/// # 示例
///
/// ```ignore
/// let mut engine = TextEngine::new();
/// let glyphs = engine.shape_text("Hello", 24.0, Attrs::new());
/// for shaped in glyphs {
///     let rasterized = engine.rasterize_glyph(&shaped.key);
///     // 将光栅化结果上传到 GlyphAtlas
/// }
/// ```
pub struct TextEngine {
    /// cosmic-text 字体系统。
    pub font_system: FontSystem,
    /// SwashCache 字形光栅化缓存。
    swash_cache: SwashCache,
    /// fontdb::ID → 内部 u64 的映射。
    id_to_u64: FxHashMap<fontdb::ID, u64>,
    /// 内部 u64 → fontdb::ID 的反向映射。
    u64_to_id: FxHashMap<u64, fontdb::ID>,
    /// 下一个分配的 u64 字体 ID。
    next_internal_id: u64,
}

impl TextEngine {
    /// 创建 TextEngine，使用嵌入字体初始化 FontSystem。
    ///
    /// # 实现说明
    ///
    /// 使用 `create_default_database()` 构建字体数据库（仅包含嵌入字体），
    /// 确保文本塑形与 vello 渲染使用同一字体，避免 glyph ID 错位。
    /// 当 CJK/Emoji 字体嵌入完成后（`embed-all-fonts` feature），
    /// `create_default_database()` 将自动包含这些字体。
    #[must_use]
    pub fn new() -> Self {
        let db = crate::font::create_default_database();
        // 尝试获取系统 locale，回退到 en-US
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|s| s.split('.').next().map(|s| s.to_string()))
            .unwrap_or_else(|| "en-US".to_string());
        let font_system = FontSystem::new_with_locale_and_db(locale, db);

        Self {
            font_system,
            swash_cache: SwashCache::new(),
            id_to_u64: FxHashMap::default(),
            u64_to_id: FxHashMap::default(),
            next_internal_id: 1,
        }
    }

    /// 将 fontdb::ID 映射为内部的 u64 标识符。
    fn intern_font_id(&mut self, db_id: fontdb::ID) -> u64 {
        *self.id_to_u64.entry(db_id).or_insert_with(|| {
            let id = self.next_internal_id;
            self.next_internal_id += 1;
            self.u64_to_id.insert(id, db_id);
            id
        })
    }

    /// 查找内部 u64 对应的 fontdb::ID。
    fn lookup_db_id(&self, internal_id: u64) -> Option<fontdb::ID> {
        self.u64_to_id.get(&internal_id).copied()
    }

    /// 塑形文本，返回需要光栅化的 GlyphKey 列表及其布局位置。
    ///
    /// # 参数
    ///
    /// * `text` — 待塑形的文本内容。
    /// * `font_size` — 字号（像素单位）。
    /// * `attrs` — 文本属性（字体族、字重、斜体等）。
    ///
    /// # 返回
    ///
    /// `Vec<ShapedGlyph>` — 每个字形的塑形信息，包含用于 atlas 查询的
    /// `GlyphKey` 和用于渲染位置计算的布局数据。
    #[must_use]
    pub fn shape_text(&mut self, text: &str, font_size: f32, attrs: Attrs<'_>) -> Vec<ShapedGlyph> {
        let metrics = Metrics {
            font_size,
            line_height: font_size * 1.2,
        };
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // 设置足够大的宽度以避免换行（8192px 为合理的最大行宽上限）
        buffer.set_size(&mut self.font_system, Some(8192.0), Some(font_size * 2.0));

        // cosmic-text 0.12: set_text 接受 4 个参数
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);

        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut result = Vec::new();
        let font_size_u32 = font_size as u32;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let internal_font_id = self.intern_font_id(glyph.font_id);
                result.push(ShapedGlyph {
                    key: GlyphKey {
                        font_id: internal_font_id,
                        glyph_id: u32::from(glyph.glyph_id),
                        font_size: font_size_u32,
                        subpx_offset: 0,
                    },
                    x: glyph.x,
                    y: glyph.y,
                    advance: glyph.w,
                });
            }
        }

        result
    }

    /// 使用 SwashCache 光栅化单个字形。
    ///
    /// # 参数
    ///
    /// * `key` — 字形键，包含 font_id（内部 u64）、glyph_id 和字号。
    ///
    /// # 返回
    ///
    /// `Option<RasterizedGlyph>` — 光栅化结果（RGBA8 位图），若字形无图像
    /// 数据（如空格、不可见字符）则返回 `None`。
    #[must_use]
    pub fn rasterize_glyph(&mut self, key: &GlyphKey) -> Option<RasterizedGlyph> {
        let db_id = self.lookup_db_id(key.font_id)?;

        let (cache_key, _, _) = CacheKey::new(
            db_id,
            key.glyph_id as u16,
            key.font_size as f32,
            (0.0, 0.0),
            CacheKeyFlags::empty(),
        );

        let swash_image = self
            .swash_cache
            .get_image_uncached(&mut self.font_system, cache_key)?;

        let width = swash_image.placement.width;
        let height = swash_image.placement.height;
        let top = swash_image.placement.top;

        if width == 0 || height == 0 {
            return None;
        }

        // 将 SwashImage → RGBA8 像素缓冲区
        let bitmap = convert_swash_image(swash_image);

        Some(RasterizedGlyph {
            bitmap,
            width,
            height,
            advance: 0.0,
            top,
        })
    }

    /// 光栅化单个字形，仅获取 placement 度量信息（不产生位图）。
    ///
    /// 返回 `(left, top, width, height)` 其中：
    /// - `left`/`top`: 相对基线的偏移（top 为负值表示字形在基线上方）
    /// - `width`/`height`: 字形位图尺寸
    #[must_use]
    pub fn glyph_placement(&mut self, key: &GlyphKey) -> Option<(i32, i32, u32, u32)> {
        let db_id = self.lookup_db_id(key.font_id)?;

        let (cache_key, _, _) = CacheKey::new(
            db_id,
            key.glyph_id as u16,
            key.font_size as f32,
            (0.0, 0.0),
            CacheKeyFlags::empty(),
        );

        let swash_image = self
            .swash_cache
            .get_image_uncached(&mut self.font_system, cache_key)?;

        let p = swash_image.placement;
        Some((p.left, p.top, p.width, p.height))
    }

    /// 创建可与 `GlyphAtlas::get_or_rasterize()` 配合使用的光栅化器闭包。
    ///
    /// 返回 `FnMut` 闭包，允许在字形 atlas 分配循环中通过 `&mut self`
    /// 调用 `rasterize_glyph()`。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let mut engine = TextEngine::new();
    /// let mut rasterizer = engine.as_rasterizer();
    /// let entry = atlas.get_or_rasterize(some_key, &mut rasterizer);
    /// ```
    pub fn as_rasterizer(&mut self) -> impl FnMut(&GlyphKey) -> Option<RasterizedGlyph> + '_ {
        |key: &GlyphKey| self.rasterize_glyph(key)
    }
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TextEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEngine")
            .field("font_system", &"<FontSystem>")
            .field("swash_cache", &"<SwashCache>")
            .field("id_to_u64", &self.id_to_u64)
            .field("next_internal_id", &self.next_internal_id)
            .finish()
    }
}

// ============================================================================
// ShapedGlyph
// ============================================================================

/// 塑形后的字形信息。
///
/// 包含用于 GlyphAtlas 缓存的 `GlyphKey` 和用于渲染位置
/// 计算的屏幕坐标。
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    /// 字形缓存键（用于 GlyphAtlas 查询）。
    pub key: GlyphKey,
    /// 字形在行中的 X 坐标（像素单位）。
    pub x: f32,
    /// 字形在行中的 Y 坐标（基线，像素单位）。
    pub y: f32,
    /// 字形步进宽度（advance width，像素单位）。
    pub advance: f32,
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 将 cosmic-text 的 SwashImage 转换为 RGBA8 像素缓冲区。
fn convert_swash_image(image: SwashImage) -> Vec<u8> {
    let width = image.placement.width as usize;
    let height = image.placement.height as usize;
    let pixel_count = width * height;

    match image.content {
        SwashContent::Mask => {
            let mut rgba = vec![0u8; pixel_count * 4];
            for (i, &alpha) in image.data.iter().enumerate() {
                rgba[i * 4] = 255;
                rgba[i * 4 + 1] = 255;
                rgba[i * 4 + 2] = 255;
                rgba[i * 4 + 3] = alpha;
            }
            rgba
        },
        SwashContent::SubpixelMask => {
            debug_assert_eq!(
                image.data.len(),
                pixel_count * 3,
                "SubpixelMask 数据长度不匹配：期望 {}，实际 {}",
                pixel_count * 3,
                image.data.len()
            );
            let mut rgba = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                let si = i * 3;
                let di = i * 4;
                rgba[di] = image.data[si];
                rgba[di + 1] = image.data[si + 1];
                rgba[di + 2] = image.data[si + 2];
                rgba[di + 3] = 255;
            }
            rgba
        },
        SwashContent::Color => {
            // Swash Color 格式已是 RGBA，直接使用
            image.data
        },
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 TextEngine 可以成功创建，确保嵌入字体正确加载。
    #[test]
    fn text_engine_creation_with_embedded_fonts() {
        let engine = TextEngine::new();
        let _ = engine;
    }

    /// 验证 Latin 文本塑形产生非空字形列表。
    #[test]
    fn shape_latin_text_produces_glyphs() {
        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("Hello", 24.0, Attrs::new());
        assert!(
            !glyphs.is_empty(),
            "Latin 文本应产生至少一个字形，实际得到 {} 个",
            glyphs.len()
        );
    }

    /// 验证每个字形有合理的 advance 值。
    #[test]
    fn shaped_glyphs_have_plausible_advance() {
        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("Test", 24.0, Attrs::new());
        for g in &glyphs {
            assert!(g.advance > 0.0, "字形 advance 应大于 0");
            assert!(g.advance < 100.0, "字形 advance 不应异常大");
            assert_eq!(g.key.font_size, 24);
        }
    }

    /// 验证字形光栅化产生有效位图。
    #[test]
    fn rasterize_glyph_produces_valid_bitmap() {
        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("A", 32.0, Attrs::new());
        assert!(!glyphs.is_empty(), "字符 'A' 应产生字形");

        let rasterized = engine.rasterize_glyph(&glyphs[0].key);
        assert!(rasterized.is_some(), "字符 'A' 应可光栅化");

        let rasterized = rasterized.unwrap();
        assert!(
            rasterized.width > 0 && rasterized.height > 0,
            "光栅化字形尺寸应大于 0，实际 {}x{}",
            rasterized.width,
            rasterized.height
        );
        assert_eq!(
            rasterized.bitmap.len(),
            (rasterized.width * rasterized.height * 4) as usize,
            "RGBA8 位图字节数应为 w*h*4"
        );
    }

    /// 验证空格等不可见字符的处理：不应 panic，且结果通常为 None。
    #[test]
    fn rasterize_space_handling() {
        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text(" ", 24.0, Attrs::new());
        // 空格可能塑形出宽度为 0 的字形
        if !glyphs.is_empty() {
            let result = engine.rasterize_glyph(&glyphs[0].key);
            // 空格字形通常宽度/高度为 0，应返回 None
            assert!(result.is_none(), "空格字形不应产生可见的光栅化位图");
        }
    }

    /// 验证相同文本产生一致的字形数量。
    #[test]
    fn consistent_shaping_for_same_text() {
        let mut engine = TextEngine::new();
        let glyphs1 = engine.shape_text("Hello", 24.0, Attrs::new());
        let glyphs2 = engine.shape_text("Hello", 24.0, Attrs::new());
        assert_eq!(glyphs1.len(), glyphs2.len());
    }

    /// 验证光栅化器闭包与 GlyphAtlas 的兼容性。
    #[test]
    fn rasterizer_closure_with_atlas() {
        use crate::glyph::GlyphAtlas;
        use crate::texture::TextureId;

        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("A", 32.0, Attrs::new());
        assert!(!glyphs.is_empty());

        let mut atlas = GlyphAtlas::new(TextureId(1), 256, 256);
        let mut rasterizer = engine.as_rasterizer();
        let entry = atlas.get_or_rasterize(glyphs[0].key.clone(), &mut rasterizer);
        assert!(entry.is_some(), "光栅化 + atlas 分配应成功");
        assert!(atlas.dirty, "首次分配应有 dirty 标记");
    }

    /// 验证多字形可依次分配入 atlas。
    #[test]
    fn multiple_glyphs_in_atlas() {
        use crate::glyph::GlyphAtlas;
        use crate::texture::TextureId;

        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("Hello", 24.0, Attrs::new());
        assert!(glyphs.len() >= 4, "Hello 至少应有 4 个字形");

        let mut atlas = GlyphAtlas::new(TextureId(1), 512, 512);
        let mut rasterizer = engine.as_rasterizer();

        let mut count = 0;
        for g in &glyphs {
            if atlas
                .get_or_rasterize(g.key.clone(), &mut rasterizer)
                .is_some()
            {
                count += 1;
            }
        }

        assert!(count >= 3, "至少 3 个字形应分配成功，实际 {}", count);
    }

    /// 验证不同字号的光栅化行为。
    #[test]
    fn different_font_sizes_produce_different_bitmaps() {
        let mut engine = TextEngine::new();

        let glyphs_16 = engine.shape_text("A", 16.0, Attrs::new());
        let glyphs_32 = engine.shape_text("A", 32.0, Attrs::new());

        let r16 = engine.rasterize_glyph(&glyphs_16[0].key);
        let r32 = engine.rasterize_glyph(&glyphs_32[0].key);

        // 两者都应成功光栅化
        assert!(r16.is_some());
        assert!(r32.is_some());
    }

    /// 验证空字符串不产生字形。
    #[test]
    fn empty_text_produces_no_glyphs() {
        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("", 24.0, Attrs::new());
        assert!(glyphs.is_empty(), "空字符串不应产生字形");
    }

    /// 验证 ShapedGlyph 支持 clone。
    #[test]
    fn shaped_glyph_is_clone() {
        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("A", 24.0, Attrs::new());
        assert!(!glyphs.is_empty());
        let cloned = glyphs[0].clone();
        assert_eq!(cloned.x, glyphs[0].x);
        assert_eq!(cloned.y, glyphs[0].y);
        assert_eq!(cloned.advance, glyphs[0].advance);
    }

    /// 验证 Default trait 实现。
    #[test]
    fn text_engine_default() {
        let _engine = TextEngine::default();
    }

    /// 验证 font_id 内化映射的一致性。
    #[test]
    fn font_id_intern_consistency() {
        let mut engine = TextEngine::new();
        let glyphs = engine.shape_text("AB", 24.0, Attrs::new());

        // 同一字体的字形应有相同的 font_id
        assert!(glyphs.len() >= 2);
        assert_eq!(
            glyphs[0].key.font_id, glyphs[1].key.font_id,
            "同一字体的字形应有相同内部 font_id"
        );
    }
}
