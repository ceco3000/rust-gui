//! 纹理类型——TextureId、TextureData、TextureFormat。
//!
//! 定义源自 D0 §5.4 和 D3 §3.2。

use std::fmt;

/// 纹理句柄（D0 §5.4）。
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct TextureId(pub u64);

impl TextureId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for TextureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextureId({})", self.0)
    }
}

/// 纹理像素数据（D0 §5.4）。
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub format: TextureFormat,
}

impl fmt::Debug for TextureData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextureData")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

/// 纹理颜色格式（D3 §3.2）。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextureFormat {
    Rgba8,
    Bgra8,
    A8,
}
