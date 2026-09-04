//! 基础几何类型。D3 阶段 0：仅提供契约核心所需类型（Point/Size/Rect），
//! 完整样式枚举（AlignContent/JustifyContent/FlexDirection 等）在实现阶段补全。

/// 二维点（i32 坐标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// 尺寸（f32 长度）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// 矩形（x/y 为原点，w/h 为尺寸）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// 布局约束（min/max 尺寸），对齐 greenfield §B.1。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxConstraints {
    /// 最小尺寸。
    pub min: Size,
    /// 最大尺寸。
    pub max: Size,
}

impl BoxConstraints {
    /// 构造布局约束。
    pub const fn new(min: Size, max: Size) -> Self {
        Self { min, max }
    }

    /// 无约束（max 视作无限，D3 占位，用 f32 上限近似）。
    pub fn loose() -> Self {
        Self {
            min: Size::default(),
            max: Size::new(f32::MAX, f32::MAX),
        }
    }
}
