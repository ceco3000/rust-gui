//! 字形 Atlas 管理。
//!
//! GPU 纹理中缓存已光栅化的字形。使用 skyline bin-packing 分配空间，
//! LRU 淘汰冷字形。通过 cosmic-text 进行字形光栅化。

use rustc_hash::FxHashMap;

use crate::skyline::{Allocation, SkylineAllocator};
use crate::texture::TextureId;

/// 字形缓存键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: u64,
    pub glyph_id: u32,
    pub font_size: u32,
    pub subpx_offset: u8,
}

/// 光栅化后的字形数据。
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub advance: f32,
}

/// 缓存中的字形条目。
#[derive(Debug, Clone)]
pub struct GlyphCacheEntry {
    pub atlas_u: f32,
    pub atlas_v: f32,
    pub width: f32,
    pub height: f32,
    pub advance: f32,
}

/// 待上传的纹理区域。
#[derive(Debug, Clone)]
pub struct UploadRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// 字形 Atlas：GPU 纹理，缓存已光栅化的字形。
///
/// 使用 bottom-left skyline bin-packing 算法。空间不足时自动增长
/// 或淘汰 LRU 冷字形。
pub struct GlyphAtlas {
    pub texture_id: TextureId,
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
    glyphs: FxHashMap<GlyphKey, GlyphCacheEntry>,
    skyline: SkylineAllocator,
    lru_counter: FxHashMap<GlyphKey, u64>,
    frame_index: u64,
    pub dirty: bool,
    pub upload_queue: Vec<UploadRect>,
}

impl GlyphAtlas {
    pub fn new(texture_id: TextureId, initial_w: u32, initial_h: u32) -> Self {
        Self {
            texture_id,
            width: initial_w,
            height: initial_h,
            max_width: 4096,
            max_height: 4096,
            glyphs: FxHashMap::default(),
            skyline: SkylineAllocator::new(initial_w, initial_h),
            lru_counter: FxHashMap::default(),
            frame_index: 0,
            dirty: false,
            upload_queue: Vec::new(),
        }
    }

    /// 请求字形缓存。命中则直接返回；未命中则调用 rasterizer
    /// 光栅化并分配 Atlas 空间。
    pub fn get_or_rasterize(
        &mut self,
        key: GlyphKey,
        rasterizer: &dyn Fn(&GlyphKey) -> Option<RasterizedGlyph>,
    ) -> Option<GlyphCacheEntry> {
        if let Some(entry) = self.glyphs.get(&key) {
            self.lru_counter.insert(key.clone(), self.frame_index);
            return Some(entry.clone());
        }

        let rasterized = rasterizer(&key)?;

        let alloc = self
            .allocate_space(rasterized.width, rasterized.height)
            .or_else(|| {
                self.grow(rasterized.width, rasterized.height);
                self.allocate_space(rasterized.width, rasterized.height)
            })
            .or_else(|| {
                self.evict_lru();
                self.allocate_space(rasterized.width, rasterized.height)
            })?;

        self.upload_queue.push(UploadRect {
            x: alloc.x,
            y: alloc.y,
            width: rasterized.width,
            height: rasterized.height,
            data: rasterized.bitmap,
        });
        self.dirty = true;

        let entry = GlyphCacheEntry {
            atlas_u: alloc.x as f32 / self.width as f32,
            atlas_v: alloc.y as f32 / self.height as f32,
            width: rasterized.width as f32,
            height: rasterized.height as f32,
            advance: rasterized.advance,
        };

        self.glyphs.insert(key.clone(), entry.clone());
        self.lru_counter.insert(key, self.frame_index);
        Some(entry)
    }

    pub fn advance_frame(&mut self) {
        self.frame_index += 1;
    }

    pub fn clear_upload_queue(&mut self) {
        self.upload_queue.clear();
        self.dirty = false;
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    fn allocate_space(&mut self, w: u32, h: u32) -> Option<Allocation> {
        self.skyline.allocate(w, h)
    }

    fn grow(&mut self, needed_w: u32, needed_h: u32) {
        if self.width < self.max_width {
            self.width = (self.width * 2).min(self.max_width).max(needed_w);
        }
        if self.height < self.max_height {
            self.height = (self.height * 2).min(self.max_height).max(needed_h);
        }
        self.skyline.resize(self.width, self.height);
    }

    fn evict_lru(&mut self) {
        let oldest = self
            .lru_counter
            .iter()
            .min_by_key(|(_, &frame)| frame)
            .map(|(k, _)| k.clone());

        if let Some(key) = oldest {
            if let Some(entry) = self.glyphs.remove(&key) {
                self.lru_counter.remove(&key);
                self.skyline.free(
                    (entry.atlas_u * self.width as f32) as u32,
                    (entry.atlas_v * self.height as f32) as u32,
                    entry.width as u32,
                    entry.height as u32,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::TextureId;

    fn dummy_rasterizer(key: &GlyphKey) -> Option<RasterizedGlyph> {
        let size = key.font_size.max(1);
        let len = (size * size) as usize;
        Some(RasterizedGlyph {
            bitmap: vec![0u8; len],
            width: size,
            height: size,
            advance: size as f32 * 0.6,
        })
    }

    #[test]
    fn caches_on_repeat_key() {
        let mut atlas = GlyphAtlas::new(TextureId(1), 512, 512);
        let key = GlyphKey {
            font_id: 1,
            glyph_id: 65,
            font_size: 32,
            subpx_offset: 0,
        };
        let e1 = atlas
            .get_or_rasterize(key.clone(), &dummy_rasterizer)
            .unwrap();
        let e2 = atlas.get_or_rasterize(key, &dummy_rasterizer).unwrap();
        assert!((e1.atlas_u - e2.atlas_u).abs() < f32::EPSILON);
    }

    #[test]
    fn allocates_multiple_keys() {
        let mut atlas = GlyphAtlas::new(TextureId(1), 512, 512);
        for gid in 0..10u32 {
            let key = GlyphKey {
                font_id: 1,
                glyph_id: gid,
                font_size: 24,
                subpx_offset: 0,
            };
            assert!(atlas.get_or_rasterize(key, &dummy_rasterizer).is_some());
        }
    }

    #[test]
    fn sets_dirty_on_alloc() {
        let mut atlas = GlyphAtlas::new(TextureId(1), 512, 512);
        assert!(!atlas.dirty);
        atlas
            .get_or_rasterize(
                GlyphKey {
                    font_id: 1,
                    glyph_id: 65,
                    font_size: 32,
                    subpx_offset: 0,
                },
                &dummy_rasterizer,
            )
            .unwrap();
        assert!(atlas.dirty);
    }

    #[test]
    fn advance_frame_works() {
        let mut atlas = GlyphAtlas::new(TextureId(1), 256, 256);
        atlas.advance_frame();
        atlas.advance_frame();
        let key = GlyphKey {
            font_id: 1,
            glyph_id: 66,
            font_size: 16,
            subpx_offset: 0,
        };
        assert!(atlas.get_or_rasterize(key, &dummy_rasterizer).is_some());
    }
}
