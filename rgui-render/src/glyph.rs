//! 字形 Atlas 管理。
//!
//! 占位模块——完整实现需 cosmic-text + wgpu/vello 集成（D3 §8）。

use crate::texture::TextureId;

/// 字形 Atlas。
///
/// 占位类型。完整实现负责：
/// - 字形光栅化（cosmic-text）
/// - Atlas 纹理分配/增长/淘汰
/// - GPU 上传（wgpu）
pub struct GlyphAtlas {
    /// Atlas 纹理 ID。
    pub texture_id: TextureId,
}

impl GlyphAtlas {
    #[must_use]
    pub fn new(texture_id: TextureId) -> Self {
        Self { texture_id }
    }
}

impl std::fmt::Debug for GlyphAtlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlyphAtlas")
            .field("texture_id", &self.texture_id)
            .finish()
    }
}
