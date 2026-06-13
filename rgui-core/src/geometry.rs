//! 基础几何类型——Point、Size、Rect 和 BoxConstraints。
//!
//! 这些类型是框架各层之间的通用几何抽象。
//! 所有坐标和尺寸使用 `f64` 以保持精度，
//! 与渲染后端（vello、cosmic-text）的坐标系统一致。

use std::fmt;

// ============================================================================
// Point
// ============================================================================

/// 二维坐标点。
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// 原点 `(0, 0)` 常量。
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// 根据坐标创建 Point。
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// 计算两点之间的欧几里得距离。
    #[must_use]
    pub fn distance_to(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// 将 x、y 坐标放大 scale_factor 倍。
    #[must_use]
    pub fn scale(self, scale_factor: f64) -> Self {
        Self {
            x: self.x * scale_factor,
            y: self.y * scale_factor,
        }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// ============================================================================
// Size
// ============================================================================

/// 二维尺寸。
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    /// 零尺寸常量。
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    /// 根据宽高创建 Size。
    #[must_use]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// 判断尺寸是否无效（任一维度 ≤ 0 或 NaN）。
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0 || self.width.is_nan() || self.height.is_nan()
    }

    /// 面积。
    #[must_use]
    pub fn area(self) -> f64 {
        self.width * self.height
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}×{}", self.width, self.height)
    }
}

// ============================================================================
// Rect
// ============================================================================

/// 轴对齐矩形，由左上角原点（origin）和尺寸（size）定义。
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    /// 零矩形常量。
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    /// 根据左上角坐标和尺寸创建 Rect。
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// 根据左上角和右下角坐标创建 Rect。
    #[must_use]
    pub fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            origin: Point::new(left, top),
            size: Size::new((right - left).max(0.0), (bottom - top).max(0.0)),
        }
    }

    /// 判断点是否在矩形内（含边界）。
    #[must_use]
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x <= self.origin.x + self.size.width
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.size.height
    }

    /// 计算两个矩形的并集（最小外接矩形）。
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if self.size.is_empty() {
            return other;
        }
        if other.size.is_empty() {
            return self;
        }
        let left = self.origin.x.min(other.origin.x);
        let top = self.origin.y.min(other.origin.y);
        let right = (self.origin.x + self.size.width).max(other.origin.x + other.size.width);
        let bottom = (self.origin.y + self.size.height).max(other.origin.y + other.size.height);
        Self::from_ltrb(left, top, right, bottom)
    }

    /// 计算两个矩形的交集。
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let left = self.origin.x.max(other.origin.x);
        let top = self.origin.y.max(other.origin.y);
        let right = (self.origin.x + self.size.width).min(other.origin.x + other.size.width);
        let bottom = (self.origin.y + self.size.height).min(other.origin.y + other.size.height);
        if left < right && top < bottom {
            Some(Self::from_ltrb(left, top, right, bottom))
        } else {
            None
        }
    }

    /// 矩形是否为空（宽度或高度 ≤ 0）。
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.size.is_empty()
    }

    /// 右边 X 坐标。
    #[must_use]
    pub fn right(self) -> f64 {
        self.origin.x + self.size.width
    }

    /// 底边 Y 坐标。
    #[must_use]
    pub fn bottom(self) -> f64 {
        self.origin.y + self.size.height
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rect({}, {} -> {}×{})",
            self.origin.x, self.origin.y, self.size.width, self.size.height
        )
    }
}

// ============================================================================
// BoxConstraints
// ============================================================================

/// 布局约束，定义 widget 布局时的最小和最大尺寸限制。
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BoxConstraints {
    pub min_width: f64,
    pub max_width: f64,
    pub min_height: f64,
    pub max_height: f64,
}

impl BoxConstraints {
    /// 无约束（min = 0, max = ∞）。
    pub const UNCONSTRAINED: Self = Self {
        min_width: 0.0,
        max_width: f64::INFINITY,
        min_height: 0.0,
        max_height: f64::INFINITY,
    };

    /// 根据 min/max 创建约束。
    #[must_use]
    pub const fn new(min_width: f64, max_width: f64, min_height: f64, max_height: f64) -> Self {
        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// 紧约束：min = max = size（强制固定尺寸）。
    #[must_use]
    pub const fn tight(size: Size) -> Self {
        Self {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
        }
    }

    /// 松约束：min = 0, max = size。
    #[must_use]
    pub const fn loose(size: Size) -> Self {
        Self {
            min_width: 0.0,
            max_width: size.width,
            min_height: 0.0,
            max_height: size.height,
        }
    }

    /// 将尺寸约束到本约束范围内。
    #[must_use]
    pub fn constrain(self, size: Size) -> Size {
        Size::new(
            size.width.clamp(self.min_width, self.max_width),
            size.height.clamp(self.min_height, self.max_height),
        )
    }

    /// 将约束放大 scale_factor 倍。
    #[must_use]
    pub fn scale(self, scale_factor: f64) -> Self {
        Self {
            min_width: self.min_width * scale_factor,
            max_width: self.max_width * scale_factor,
            min_height: self.min_height * scale_factor,
            max_height: self.max_height * scale_factor,
        }
    }
}

impl Default for BoxConstraints {
    fn default() -> Self {
        Self::UNCONSTRAINED
    }
}

impl fmt::Display for BoxConstraints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BoxConstraints(w: {}→{}, h: {}→{})",
            self.min_width, self.max_width, self.min_height, self.max_height
        )
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Point ---

    #[test]
    fn point_default_is_zero() {
        assert_eq!(Point::default(), Point::ZERO);
    }

    #[test]
    fn point_distance_horizontal() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 0.0);
        assert_eq!(a.distance_to(b), 3.0);
    }

    #[test]
    fn point_distance_diagonal() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert_eq!(a.distance_to(b), 5.0);
    }

    #[test]
    fn point_scale() {
        let p = Point::new(1.0, 2.0).scale(2.0);
        assert_eq!(p, Point::new(2.0, 4.0));
    }

    // --- Size ---

    #[test]
    fn size_zero_is_empty() {
        assert!(Size::ZERO.is_empty());
    }

    #[test]
    fn size_positive_is_not_empty() {
        assert!(!Size::new(100.0, 200.0).is_empty());
    }

    #[test]
    fn size_nan_is_empty() {
        assert!(Size::new(f64::NAN, 100.0).is_empty());
    }

    #[test]
    fn size_area() {
        assert_eq!(Size::new(3.0, 4.0).area(), 12.0);
    }

    // --- Rect ---

    #[test]
    fn rect_contains_center_point() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(rect.contains(Point::new(5.0, 5.0)));
    }

    #[test]
    fn rect_contains_boundary() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(rect.contains(Point::new(0.0, 0.0)));
        assert!(rect.contains(Point::new(10.0, 10.0)));
    }

    #[test]
    fn rect_does_not_contain_outside_point() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(!rect.contains(Point::new(-1.0, 5.0)));
        assert!(!rect.contains(Point::new(5.0, 11.0)));
    }

    #[test]
    fn rect_union_overlapping() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 15.0, 15.0);
        let u = a.union(b);
        assert_eq!(u, Rect::new(0.0, 0.0, 20.0, 20.0));
    }

    #[test]
    fn rect_union_disjoint() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(20.0, 20.0, 10.0, 10.0);
        let u = a.union(b);
        assert_eq!(u, Rect::from_ltrb(0.0, 0.0, 30.0, 30.0));
    }

    #[test]
    fn rect_union_with_empty() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(a.union(Rect::ZERO), a);
    }

    #[test]
    fn rect_intersection_overlapping() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 15.0, 15.0);
        assert_eq!(a.intersection(b), Some(Rect::new(5.0, 5.0, 5.0, 5.0)));
    }

    #[test]
    fn rect_intersection_disjoint() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(20.0, 20.0, 10.0, 10.0);
        assert_eq!(a.intersection(b), None);
    }

    #[test]
    fn rect_is_empty() {
        assert!(Rect::new(0.0, 0.0, 0.0, 10.0).is_empty());
        assert!(!Rect::new(0.0, 0.0, 10.0, 10.0).is_empty());
    }

    #[test]
    fn rect_from_ltrb() {
        let r = Rect::from_ltrb(1.0, 2.0, 5.0, 7.0);
        assert_eq!(r.origin, Point::new(1.0, 2.0));
        assert_eq!(r.size, Size::new(4.0, 5.0));
    }

    #[test]
    fn rect_from_ltrb_inverted_is_empty() {
        let r = Rect::from_ltrb(5.0, 7.0, 1.0, 2.0);
        assert_eq!(r.size, Size::ZERO);
    }

    // --- BoxConstraints ---

    #[test]
    fn constraints_tight_forces_size() {
        let c = BoxConstraints::tight(Size::new(100.0, 200.0));
        let s = c.constrain(Size::new(50.0, 500.0));
        assert_eq!(s, Size::new(100.0, 200.0));
    }

    #[test]
    fn constraints_loose_allows_smaller() {
        let c = BoxConstraints::loose(Size::new(100.0, 200.0));
        let s = c.constrain(Size::new(50.0, 100.0));
        assert_eq!(s, Size::new(50.0, 100.0));
    }

    #[test]
    fn constraints_clamp_to_bounds() {
        let c = BoxConstraints::new(0.0, 100.0, 0.0, 100.0);
        let s = c.constrain(Size::new(200.0, 300.0));
        assert_eq!(s, Size::new(100.0, 100.0));
    }

    #[test]
    fn constraints_scale() {
        let c = BoxConstraints::new(10.0, 100.0, 20.0, 200.0).scale(2.0);
        assert_eq!(c.min_width, 20.0);
        assert_eq!(c.max_width, 200.0);
        assert_eq!(c.min_height, 40.0);
        assert_eq!(c.max_height, 400.0);
    }
}
