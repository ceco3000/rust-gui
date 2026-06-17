//! 基础几何类型——Point、Size、Rect 和 BoxConstraints。
//!
//! 这些类型是框架各层之间的通用几何抽象。
//! 所有坐标和尺寸使用 `f64` 以保持精度，
//! 与渲染后端（vello、cosmic-text）的坐标系统一致。
//!
//! # 坐标系统
//!
//! **逻辑像素（device-independent pixels）** —— 本模块中所有类型
//! （`Point`、`Size`、`Rect`、`BoxConstraints`）均以逻辑像素为单位。
//!
//! ## 契约
//!
//! | 层级 | 坐标系统 | 说明 |
//! |------|---------|------|
//! | 组件层（`paint()`、`measure()`、`hit_test()`） | 逻辑像素 | 组件代码无需感知物理像素 |
//! | 平台适配层（`rgui-platform` 事件转换） | 物理→逻辑转换 | `winit` 返回的是物理像素，需除以 `scale_factor` 转为逻辑像素 |
//! | 渲染后端边界（`RenderBackend` trait） | 物理像素 | `RenderParams.width/height` 为物理像素；`scale_factor` 字段供后端在编码时做坐标变换 |
//!
//! ## 物理像素 → 逻辑像素转换
//!
//! 在 `convert_winit_event()` 等 winit 事件转换点，`CursorMoved` 的
//! `PhysicalPosition` 必须除以 `window.scale_factor()` 得到逻辑 `Point`，
//! 再传入 `HitTester` 和 `EventRouter`。否则在高 DPI 显示器上命中测试
//! 坐标会偏移。
//!
//! ## 逻辑像素 → 物理像素转换
//!
//! 布局和 paint 使用逻辑像素，渲染后端在编码时通过 `scale_factor`
//! 将逻辑坐标放大为物理坐标。`Point::scale(scale_factor)` 和
//! `BoxConstraints::scale(scale_factor)` 提供便捷方法。

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
// 布局相关枚举
// ============================================================================

/// 布局显示类型。
///
/// 对应 CSS `display` 属性，控制元素使用何种布局模式。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum LayoutDisplay {
    /// Flex 布局。
    Flex,
    /// Grid 布局。
    Grid,
    /// 块级布局。
    #[default]
    Block,
    /// 隐藏（不参与布局）。
    None,
}

/// Flex 主轴方向。
///
/// 对应 CSS `flex-direction` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FlexDirection {
    /// 主轴水平，从左到右。
    #[default]
    Row,
    /// 主轴水平，从右到左。
    RowReverse,
    /// 主轴垂直，从上到下。
    Column,
    /// 主轴垂直，从下到上。
    ColumnReverse,
}

/// 主轴对齐方式。
///
/// 对应 CSS `justify-content` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum JustifyContent {
    /// 起点对齐。
    #[default]
    Start,
    /// 终点对齐。
    End,
    /// 居中对齐。
    Center,
    /// 两端对齐，项目之间间距相等。
    SpaceBetween,
    /// 项目周围间距相等。
    SpaceAround,
    /// 项目之间和两端间距相等。
    SpaceEvenly,
}

/// 交叉轴对齐方式。
///
/// 对应 CSS `align-items` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum AlignItems {
    /// 起点对齐。
    #[default]
    Start,
    /// 终点对齐。
    End,
    /// 居中对齐。
    Center,
    /// 基线对齐。
    Baseline,
    /// 拉伸填充。
    Stretch,
}

/// 交叉轴内容对齐方式。
///
/// 对应 CSS `align-content` 属性，控制多行/多列在交叉轴上的分布。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum AlignContent {
    /// 起点对齐。
    #[default]
    Start,
    /// 终点对齐。
    End,
    /// 居中对齐。
    Center,
    /// 两端对齐。
    SpaceBetween,
    /// 项目周围间距相等。
    SpaceAround,
    /// 项目之间和两端间距相等。
    SpaceEvenly,
    /// 拉伸填充。
    Stretch,
}

// ============================================================================
// FlexWrap
// ============================================================================

/// Flex 换行模式。
///
/// 对应 CSS `flex-wrap` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FlexWrap {
    /// 不换行（默认）。
    #[default]
    NoWrap,
    /// 换行，第一行在上方。
    Wrap,
    /// 换行，第一行在下方。
    WrapReverse,
}

// ============================================================================
// AlignSelf
// ============================================================================

/// 单个子元素的交叉轴对齐方式（覆盖父元素 `align-items`）。
///
/// 对应 CSS `align-self` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum AlignSelf {
    /// 继承父元素 `align-items`。
    #[default]
    Auto,
    /// 起点对齐。
    Start,
    /// 终点对齐。
    End,
    /// 居中对齐。
    Center,
    /// 基线对齐。
    Baseline,
    /// 拉伸填充。
    Stretch,
}

// ============================================================================
// FlexBasis
// ============================================================================

/// Flex 子元素的初始主轴尺寸。
///
/// 对应 CSS `flex-basis` 属性。
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub enum FlexBasis {
    /// 自动确定（基于内容或 width/height）。
    #[default]
    Auto,
    /// 固定长度（逻辑像素）。
    Length(f64),
    /// 百分比（相对于父元素主轴尺寸）。
    Percent(f64),
}

// ============================================================================
// GridTrack
// ============================================================================

/// CSS Grid 轨道定义（列或行）。
///
/// 对应 CSS `grid-template-columns` / `grid-template-rows` 属性中的单个轨道值。
#[derive(Clone, PartialEq, Debug)]
pub enum GridTrack {
    /// 固定像素宽度/高度。
    Px(f64),
    /// 弹性分数（`fr` 单位）。
    Fr(f64),
    /// 内容自适应。
    Auto,
    /// 百分比。
    Percent(f64),
    /// 最小值-最大值范围（`minmax()` 函数）。
    MinMax(Box<GridTrack>, Box<GridTrack>),
}

// ============================================================================
// LayoutStyle
// ============================================================================

/// 布局样式——CSS 布局属性到 Taffy 布局引擎的类型化中间层。
///
/// `LayoutStyle` 是 `rgui-style`（产生 CSS 属性键值对）与 `rgui-layout`
///（需要 Taffy `Style`）之间的交互契约。它定义于 `rgui-core::geometry`
/// 模块，确保两个子系统共享同一布局属性集而无需互相依赖。
///
/// 所有属性均为 `Option`——`None` 表示该属性未被设置，布局引擎将使用默认值。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutStyle {
    /// 显示类型（flex / grid / block / none）。
    pub display: Option<LayoutDisplay>,
    /// 宽度（逻辑像素）。
    pub width: Option<f64>,
    /// 高度（逻辑像素）。
    pub height: Option<f64>,
    /// 最小宽度。
    pub min_width: Option<f64>,
    /// 最小高度。
    pub min_height: Option<f64>,
    /// 最大宽度。
    pub max_width: Option<f64>,
    /// 最大高度。
    pub max_height: Option<f64>,
    /// Flex 主轴方向。
    pub flex_direction: Option<FlexDirection>,
    /// 主轴对齐方式。
    pub justify_content: Option<JustifyContent>,
    /// 交叉轴对齐方式。
    pub align_items: Option<AlignItems>,
    /// 交叉轴内容对齐方式。
    pub align_content: Option<AlignContent>,
    /// 子元素间距（逻辑像素）。
    pub gap: Option<f64>,
    /// 内边距（四边相同，逻辑像素）。
    pub padding: Option<f64>,
    /// 外边距（四边相同，逻辑像素）。
    pub margin: Option<f64>,
    /// 换行模式。
    pub flex_wrap: Option<FlexWrap>,
    /// 单个子元素交叉轴对齐。
    pub align_self: Option<AlignSelf>,
    /// Flex 增长因子。
    pub flex_grow: Option<f32>,
    /// Flex 收缩因子。
    pub flex_shrink: Option<f32>,
    /// Flex 基础尺寸。
    pub flex_basis: Option<FlexBasis>,
    /// 宽高比（宽/高，如 16.0/9.0）。
    pub aspect_ratio: Option<f64>,
    /// Grid 模板列（每个元素对应一列）。
    pub grid_template_columns: Option<Vec<GridTrack>>,
    /// Grid 模板行（每个元素对应一行）。
    pub grid_template_rows: Option<Vec<GridTrack>>,
}

impl LayoutStyle {
    /// 创建空布局样式（所有属性均为 `None`）。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 判断是否所有属性均为 `None`（无任何布局设置）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.display.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.min_width.is_none()
            && self.min_height.is_none()
            && self.max_width.is_none()
            && self.max_height.is_none()
            && self.flex_direction.is_none()
            && self.justify_content.is_none()
            && self.align_items.is_none()
            && self.align_content.is_none()
            && self.gap.is_none()
            && self.padding.is_none()
            && self.margin.is_none()
            && self.flex_wrap.is_none()
            && self.align_self.is_none()
            && self.flex_grow.is_none()
            && self.flex_shrink.is_none()
            && self.flex_basis.is_none()
            && self.aspect_ratio.is_none()
            && self.grid_template_columns.is_none()
            && self.grid_template_rows.is_none()
    }

    /// 合并另一个 `LayoutStyle`，`other` 中的非 `None` 属性覆盖 `self` 中对应属性。
    ///
    /// 用于样式层叠——默认主题 → 组件默认 → 用户样式 → 内联样式。
    #[must_use]
    pub fn merge(self, other: &Self) -> Self {
        Self {
            display: other.display.or(self.display),
            width: other.width.or(self.width),
            height: other.height.or(self.height),
            min_width: other.min_width.or(self.min_width),
            min_height: other.min_height.or(self.min_height),
            max_width: other.max_width.or(self.max_width),
            max_height: other.max_height.or(self.max_height),
            flex_direction: other.flex_direction.or(self.flex_direction),
            justify_content: other.justify_content.or(self.justify_content),
            align_items: other.align_items.or(self.align_items),
            align_content: other.align_content.or(self.align_content),
            gap: other.gap.or(self.gap),
            padding: other.padding.or(self.padding),
            margin: other.margin.or(self.margin),
            flex_wrap: other.flex_wrap.or(self.flex_wrap),
            align_self: other.align_self.or(self.align_self),
            flex_grow: other.flex_grow.or(self.flex_grow),
            flex_shrink: other.flex_shrink.or(self.flex_shrink),
            flex_basis: other.flex_basis.or(self.flex_basis),
            aspect_ratio: other.aspect_ratio.or(self.aspect_ratio),
            grid_template_columns: other
                .grid_template_columns
                .as_ref()
                .cloned()
                .or(self.grid_template_columns),
            grid_template_rows: other
                .grid_template_rows
                .as_ref()
                .cloned()
                .or(self.grid_template_rows),
        }
    }
}

impl fmt::Display for LayoutStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LayoutStyle(")?;

        let mut first = true;
        let mut prop = |name: &str, val: &dyn fmt::Display| {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{name}: {val}")
        };

        if let Some(d) = &self.display {
            prop("display", d)?;
        }
        if let Some(w) = self.width {
            prop("width", &w)?;
        }
        if let Some(h) = self.height {
            prop("height", &h)?;
        }
        if let Some(fd) = &self.flex_direction {
            prop("flex-direction", fd)?;
        }
        if let Some(g) = self.gap {
            prop("gap", &g)?;
        }
        if let Some(p) = self.padding {
            prop("padding", &p)?;
        }
        if let Some(m) = self.margin {
            prop("margin", &m)?;
        }
        if let Some(fw) = &self.flex_wrap {
            prop("flex-wrap", fw)?;
        }
        if let Some(al) = &self.align_self {
            prop("align-self", al)?;
        }
        if let Some(fg) = self.flex_grow {
            prop("flex-grow", &fg)?;
        }
        if let Some(fs) = self.flex_shrink {
            prop("flex-shrink", &fs)?;
        }
        if let Some(mw) = self.min_width {
            prop("min-width", &mw)?;
        }
        if let Some(mh) = self.min_height {
            prop("min-height", &mh)?;
        }
        if let Some(mxw) = self.max_width {
            prop("max-width", &mxw)?;
        }
        if let Some(mxh) = self.max_height {
            prop("max-height", &mxh)?;
        }

        if first {
            write!(f, "empty")?;
        }

        write!(f, ")")
    }
}

impl fmt::Display for LayoutDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flex => write!(f, "flex"),
            Self::Grid => write!(f, "grid"),
            Self::Block => write!(f, "block"),
            Self::None => write!(f, "none"),
        }
    }
}

impl fmt::Display for FlexDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Row => write!(f, "row"),
            Self::RowReverse => write!(f, "row-reverse"),
            Self::Column => write!(f, "column"),
            Self::ColumnReverse => write!(f, "column-reverse"),
        }
    }
}

impl fmt::Display for JustifyContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::End => write!(f, "end"),
            Self::Center => write!(f, "center"),
            Self::SpaceBetween => write!(f, "space-between"),
            Self::SpaceAround => write!(f, "space-around"),
            Self::SpaceEvenly => write!(f, "space-evenly"),
        }
    }
}

impl fmt::Display for AlignItems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::End => write!(f, "end"),
            Self::Center => write!(f, "center"),
            Self::Baseline => write!(f, "baseline"),
            Self::Stretch => write!(f, "stretch"),
        }
    }
}

impl fmt::Display for AlignContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::End => write!(f, "end"),
            Self::Center => write!(f, "center"),
            Self::SpaceBetween => write!(f, "space-between"),
            Self::SpaceAround => write!(f, "space-around"),
            Self::SpaceEvenly => write!(f, "space-evenly"),
            Self::Stretch => write!(f, "stretch"),
        }
    }
}

impl fmt::Display for FlexWrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoWrap => write!(f, "nowrap"),
            Self::Wrap => write!(f, "wrap"),
            Self::WrapReverse => write!(f, "wrap-reverse"),
        }
    }
}

impl fmt::Display for AlignSelf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Start => write!(f, "start"),
            Self::End => write!(f, "end"),
            Self::Center => write!(f, "center"),
            Self::Baseline => write!(f, "baseline"),
            Self::Stretch => write!(f, "stretch"),
        }
    }
}

// ============================================================================
// VisualStyle
// ============================================================================

/// 视觉样式——控制 widget 的颜色、边框、阴影和可见性。
///
/// 对应 CSS 视觉相关属性（background-color, color, opacity,
/// border-radius, border-width, border-color, box-shadow, visibility）。
///
/// 所有属性均为 `Option`——`None` 表示该属性未被设置，渲染时将使用默认值。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VisualStyle {
    /// 背景颜色。
    pub background_color: Option<crate::view::Color>,
    /// 前景色（文本颜色）。
    pub color: Option<crate::view::Color>,
    /// 不透明度（0.0-1.0）。
    pub opacity: Option<f64>,
    /// 圆角半径（逻辑像素）。
    pub border_radius: Option<f64>,
    /// 边框宽度（逻辑像素）。
    pub border_width: Option<f64>,
    /// 边框颜色。
    pub border_color: Option<crate::view::Color>,
    /// 盒子阴影：`(x, y, blur, color)`。
    pub box_shadow: Option<(f64, f64, f64, crate::view::Color)>,
    /// 可见性（visible / hidden）。
    pub visibility: Option<Visibility>,
}

/// 可见性状态。
///
/// 对应 CSS `visibility` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Visibility {
    /// 可见（默认）。
    #[default]
    Visible,
    /// 隐藏但仍占位。
    Hidden,
}

impl VisualStyle {
    /// 创建空视觉样式（所有属性均为 `None`）。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 判断是否所有属性均为 `None`。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.background_color.is_none()
            && self.color.is_none()
            && self.opacity.is_none()
            && self.border_radius.is_none()
            && self.border_width.is_none()
            && self.border_color.is_none()
            && self.box_shadow.is_none()
            && self.visibility.is_none()
    }

    /// 合并另一个 `VisualStyle`，`other` 中的非 `None` 属性覆盖 `self` 中对应属性。
    #[must_use]
    pub fn merge(self, other: &Self) -> Self {
        Self {
            background_color: other.background_color.or(self.background_color),
            color: other.color.or(self.color),
            opacity: other.opacity.or(self.opacity),
            border_radius: other.border_radius.or(self.border_radius),
            border_width: other.border_width.or(self.border_width),
            border_color: other.border_color.or(self.border_color),
            box_shadow: other.box_shadow.or(self.box_shadow),
            visibility: other.visibility.or(self.visibility),
        }
    }
}

impl fmt::Display for VisualStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VisualStyle(")?;

        let mut first = true;
        let mut prop = |name: &str, val: &dyn fmt::Display| {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{name}: {val}")
        };

        if let Some(c) = &self.background_color {
            prop("background-color", c)?;
        }
        if let Some(c) = &self.color {
            prop("color", c)?;
        }
        if let Some(o) = self.opacity {
            prop("opacity", &o)?;
        }

        if first {
            write!(f, "empty")?;
        }

        write!(f, ")")
    }
}

// ============================================================================
// TextStyle
// ============================================================================

/// 文本样式——控制字体、字号、字重、行高、对齐等排版属性。
///
/// 对应 CSS 文本相关属性（font-family, font-size, font-weight,
/// font-style, line-height, letter-spacing, text-align,
/// text-overflow, white-space）。
///
/// 所有属性均为 `Option`——`None` 表示该属性未被设置，文本渲染时将使用默认值。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextStyle {
    /// 字体族（如 `"Inter", sans-serif`）。
    pub font_family: Option<String>,
    /// 字体大小（逻辑像素）。
    pub font_size: Option<f64>,
    /// 字重：`400`（normal）、`700`（bold）等。
    pub font_weight: Option<FontWeight>,
    /// 字体样式。
    pub font_style: Option<FontStyle>,
    /// 行高（倍数或逻辑像素）。
    pub line_height: Option<f64>,
    /// 字间距（逻辑像素）。
    pub letter_spacing: Option<f64>,
    /// 文本水平对齐方式。
    pub text_align: Option<TextAlign>,
    /// 文本溢出处理方式。
    pub text_overflow: Option<TextOverflow>,
    /// 空白字符处理方式。
    pub white_space: Option<WhiteSpace>,
}

/// 字重。
///
/// 对应 CSS `font-weight` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FontWeight {
    /// 100
    Thin,
    /// 200
    ExtraLight,
    /// 300
    Light,
    /// 400（默认）
    #[default]
    Normal,
    /// 500
    Medium,
    /// 600
    SemiBold,
    /// 700
    Bold,
    /// 800
    ExtraBold,
    /// 900
    Black,
    /// 数值字重（1-1000）。
    Number(u16),
}

/// 字体样式。
///
/// 对应 CSS `font-style` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FontStyle {
    /// 正常（默认）。
    #[default]
    Normal,
    /// 斜体。
    Italic,
    /// 倾斜体。
    Oblique,
}

/// 文本水平对齐方式。
///
/// 对应 CSS `text-align` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TextAlign {
    /// 起始对齐（左对齐 for LTR）。
    #[default]
    Start,
    /// 居中。
    Center,
    /// 结束对齐（右对齐 for LTR）。
    End,
    /// 两端对齐。
    Justify,
}

/// 文本溢出处理方式。
///
/// 对应 CSS `text-overflow` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TextOverflow {
    /// 直接裁剪（默认）。
    #[default]
    Clip,
    /// 显示省略号 `…`。
    Ellipsis,
}

/// 空白字符处理方式。
///
/// 对应 CSS `white-space` 属性。
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum WhiteSpace {
    /// 正常换行（默认）。
    #[default]
    Normal,
    /// 不换行。
    NoWrap,
    /// 保留空白和换行。
    Pre,
}

impl TextStyle {
    /// 创建空文本样式（所有属性均为 `None`）。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 判断是否所有属性均为 `None`。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.font_family.is_none()
            && self.font_size.is_none()
            && self.font_weight.is_none()
            && self.font_style.is_none()
            && self.line_height.is_none()
            && self.letter_spacing.is_none()
            && self.text_align.is_none()
            && self.text_overflow.is_none()
            && self.white_space.is_none()
    }

    /// 合并另一个 `TextStyle`，`other` 中的非 `None` 属性覆盖 `self` 中对应属性。
    #[must_use]
    pub fn merge(self, other: &Self) -> Self {
        Self {
            font_family: other.font_family.as_ref().cloned().or(self.font_family),
            font_size: other.font_size.or(self.font_size),
            font_weight: other.font_weight.or(self.font_weight),
            font_style: other.font_style.or(self.font_style),
            line_height: other.line_height.or(self.line_height),
            letter_spacing: other.letter_spacing.or(self.letter_spacing),
            text_align: other.text_align.or(self.text_align),
            text_overflow: other.text_overflow.or(self.text_overflow),
            white_space: other.white_space.or(self.white_space),
        }
    }
}

impl fmt::Display for TextStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextStyle(")?;

        let mut first = true;
        let mut prop = |name: &str, val: &dyn fmt::Display| {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{name}: {val}")
        };

        if let Some(ff) = &self.font_family {
            prop("font-family", &ff)?;
        }
        if let Some(fs) = self.font_size {
            prop("font-size", &fs)?;
        }

        if first {
            write!(f, "empty")?;
        }

        write!(f, ")")
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

    // --- LayoutDisplay ---

    #[test]
    fn layout_display_default_is_block() {
        assert_eq!(LayoutDisplay::default(), LayoutDisplay::Block);
    }

    #[test]
    fn layout_display_display_trait() {
        assert_eq!(format!("{}", LayoutDisplay::Flex), "flex");
        assert_eq!(format!("{}", LayoutDisplay::Grid), "grid");
        assert_eq!(format!("{}", LayoutDisplay::Block), "block");
        assert_eq!(format!("{}", LayoutDisplay::None), "none");
    }

    // --- FlexDirection ---

    #[test]
    fn flex_direction_default_is_row() {
        assert_eq!(FlexDirection::default(), FlexDirection::Row);
    }

    // --- JustifyContent ---

    #[test]
    fn justify_content_default_is_start() {
        assert_eq!(JustifyContent::default(), JustifyContent::Start);
    }

    // --- AlignItems ---

    #[test]
    fn align_items_default_is_start() {
        assert_eq!(AlignItems::default(), AlignItems::Start);
    }

    // --- LayoutStyle ---

    #[test]
    fn layout_style_new_is_empty() {
        let style = LayoutStyle::new();
        assert!(style.is_empty());
    }

    #[test]
    fn layout_style_default_is_empty() {
        assert!(LayoutStyle::default().is_empty());
    }

    #[test]
    fn layout_style_with_properties_is_not_empty() {
        let style = LayoutStyle {
            display: Some(LayoutDisplay::Flex),
            width: Some(200.0),
            ..LayoutStyle::default()
        };
        assert!(!style.is_empty());
    }

    #[test]
    fn layout_style_merge_baseline() {
        let base = LayoutStyle {
            display: Some(LayoutDisplay::Flex),
            width: Some(100.0),
            ..LayoutStyle::default()
        };
        let over = LayoutStyle {
            width: Some(200.0),
            height: Some(50.0),
            ..LayoutStyle::default()
        };
        let merged = base.merge(&over);
        // over 覆盖 width
        assert_eq!(merged.width, Some(200.0));
        // base 的 display 保留（over 未设置）
        assert_eq!(merged.display, Some(LayoutDisplay::Flex));
        // over 的 height 追加
        assert_eq!(merged.height, Some(50.0));
    }

    #[test]
    fn layout_style_merge_fully_overrides() {
        let base = LayoutStyle {
            display: Some(LayoutDisplay::Block),
            width: Some(100.0),
            gap: Some(4.0),
            ..LayoutStyle::default()
        };
        let over = LayoutStyle {
            display: Some(LayoutDisplay::Flex),
            width: Some(200.0),
            gap: Some(8.0),
            ..LayoutStyle::default()
        };
        let merged = base.merge(&over);
        assert_eq!(merged.display, Some(LayoutDisplay::Flex));
        assert_eq!(merged.width, Some(200.0));
        assert_eq!(merged.gap, Some(8.0));
    }

    #[test]
    fn layout_style_display_repr() {
        let style = LayoutStyle {
            display: Some(LayoutDisplay::Flex),
            width: Some(300.0),
            height: Some(200.0),
            flex_direction: Some(FlexDirection::Column),
            gap: Some(8.0),
            padding: Some(16.0),
            margin: Some(4.0),
            ..LayoutStyle::default()
        };
        let repr = format!("{style}");
        assert!(repr.starts_with("LayoutStyle("));
        assert!(repr.contains("display: flex"));
        assert!(repr.contains("width: 300"));
        assert!(repr.contains("height: 200"));
        assert!(repr.contains("flex-direction: column"));
        assert!(repr.contains("gap: 8"));
    }

    #[test]
    fn layout_style_empty_display_repr() {
        let style = LayoutStyle::new();
        let repr = format!("{style}");
        assert!(repr.contains("empty"));
    }
}
