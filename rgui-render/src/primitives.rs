//! 渲染基础类型——PathData、Paint、Stroke、GlyphData、Transform、BlendMode。
//!
//! 定义源自 D3 §3.2-§3.3。

use crate::texture::TextureId;
use rgui_core::Color;

// ============================================================================
// PathData
// ============================================================================

#[derive(Clone, Debug)]
pub struct PathData {
    pub commands: Vec<PathCommand>,
    pub fill_rule: FillRule,
}

#[derive(Clone, Debug)]
pub enum PathCommand {
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    QuadTo {
        cx: f32,
        cy: f32,
        x: f32,
        y: f32,
    },
    CubicTo {
        cx1: f32,
        cy1: f32,
        cx2: f32,
        cy2: f32,
        x: f32,
        y: f32,
    },
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

// ============================================================================
// Paint
// ============================================================================

#[derive(Clone, Debug)]
pub enum Paint {
    Solid(Color),
    LinearGradient {
        start: Point,
        end: Point,
        stops: Vec<GradientStop>,
    },
    RadialGradient {
        center: Point,
        radius: f32,
        stops: Vec<GradientStop>,
    },
    Image {
        texture_id: TextureId,
        repeat: ImageRepeat,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Debug)]
pub struct GradientStop {
    pub position: f32,
    pub color: Color,
}

#[derive(Copy, Clone, Debug)]
pub enum ImageRepeat {
    NoRepeat,
    Repeat,
    RepeatX,
    RepeatY,
}

// ============================================================================
// Stroke
// ============================================================================

#[derive(Clone, Debug)]
pub struct Stroke {
    pub width: f32,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f32,
    pub dash_pattern: Option<Vec<f32>>,
    pub dash_offset: f32,
}

#[derive(Copy, Clone, Debug)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Copy, Clone, Debug)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

// ============================================================================
// GlyphData
// ============================================================================

/// 字形绘制数据（D3 §3.2）。
#[derive(Clone, Debug)]
pub struct GlyphData {
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub atlas_w: u32,
    pub atlas_h: u32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub advance: f32,
    /// skrifa glyph ID，供 vello draw_glyphs() 使用
    pub glyph_index: u32,
}

// ============================================================================
// Transform
// ============================================================================

/// 2D 仿射变换矩阵（D3 §3.2）。
#[derive(Clone, Debug)]
pub struct Transform {
    pub matrix: [f32; 6],
}

impl Transform {
    #[must_use]
    pub fn identity() -> Self {
        Self {
            matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        }
    }

    #[must_use]
    pub fn translate(dx: f32, dy: f32) -> Self {
        Self {
            matrix: [1.0, 0.0, dx, 0.0, 1.0, dy],
        }
    }

    #[must_use]
    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            matrix: [sx, 0.0, 0.0, 0.0, sy, 0.0],
        }
    }
}

// ============================================================================
// BlendMode
// ============================================================================

#[derive(Copy, Clone, Debug)]
pub enum BlendMode {
    SrcOver,
    Src,
    Multiply,
    Screen,
    Overlay,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_identity() {
        let t = Transform::identity();
        assert_eq!(t.matrix[0], 1.0);
    }

    #[test]
    fn transform_translate() {
        let t = Transform::translate(10.0, 20.0);
        assert_eq!(t.matrix[2], 10.0);
        assert_eq!(t.matrix[5], 20.0);
    }

    #[test]
    fn glyph_data_construct() {
        let g = GlyphData {
            atlas_x: 0,
            atlas_y: 0,
            atlas_w: 16,
            atlas_h: 16,
            offset_x: 1.0,
            offset_y: 2.0,
            advance: 14.0,
            glyph_index: 0,
        };
        assert_eq!(g.atlas_w, 16);
    }
}
