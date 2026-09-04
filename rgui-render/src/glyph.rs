//! 字形模块（GPU 资源类型，契约 §2.4）。
//! D3 阶段 0：占位类型定义，真实字形缓存/atlas 在实现阶段补全。

/// 字形缓存项。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphCacheEntry(pub u64);

/// 字形键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GlyphKey {
    pub glyph_id: u32,
    pub font_id: u32,
}

/// 字形图集。
#[derive(Debug, Clone, Default)]
pub struct GlyphAtlas {
    _marker: std::marker::PhantomData<()>,
}
