//! skrifa 文本塑形模块——为 vello draw_glyphs() 提供 glyph ID + 布局坐标。
//!
//! 与 cosmic-text TextEngine 并行存在：
//! - TextEngine（cosmic-text）：CJK 回退、系统字体支持（未来使用）
//! - SkrifaShaper（本模块）：拉丁文字塑形，产出 vello 兼容的 glyph ID

use skrifa::{GlyphId, MetadataProvider};

/// skrifa 文本塑形器。
///
/// 从嵌入的 Inter 字体创建，提供拉丁文字的基本塑形能力。
pub struct SkrifaShaper {
    font_ref: skrifa::FontRef<'static>, // 从 include_bytes! 创建，'static 生命周期
}

impl SkrifaShaper {
    /// 从嵌入字体创建塑形器。
    #[must_use]
    pub fn new() -> Self {
        let font_bytes: &'static [u8] =
            include_bytes!("../../assets/fonts/Inter-Regular.ttf");
        // FontRef::new 返回 Result，'static 字节总是有效
        let font_ref = skrifa::FontRef::new(font_bytes).expect("Inter 字体加载失败");
        Self { font_ref }
    }

    /// 塑形文本，返回 (glyph_id, x_offset, y_offset, advance_width) 列表。
    #[must_use]
    pub fn shape_text(&self, text: &str, font_size: f32) -> Vec<(u32, f32, f32, f32)> {
        let mut cursor_x = 0.0_f32;
        let mut result = Vec::new();

        for ch in text.chars() {
            // 通过 cmap 获取字符对应的 glyph ID
            let glyph_id = self
                .font_ref
                .charmap()
                .map(ch)
                .unwrap_or(GlyphId::NOTDEF);

            let id = u32::from(glyph_id);

            // 计算简单水平布局（不考虑 kerning，Latin 基本够用）
            result.push((id, cursor_x, 0.0, font_size * 0.6));

            // 粗略 advance：等宽 0.6em（Inter 拉丁字母接近等宽比例）
            cursor_x += font_size * 0.6;
        }
        result
    }
}

impl Default for SkrifaShaper {
    fn default() -> Self {
        Self::new()
    }
}
