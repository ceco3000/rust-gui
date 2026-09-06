//! 布局样式映射：`LayoutStyle → Taffy Style` 封装（greenfield §C.1 layout/mapping.rs）。
//!
//! **不把 Taffy 类型泄漏到公共 API**：`LayoutStyle` 为纯 Rust 公共类型，`to_taffy_style`
//! 仅在 `#[cfg(feature="layout")]` 下实现（依赖 taffy）。Taffy 是纯 Rust，无平台/GPU。

/// 布局方向（D23 残留 P1-1：容器内部主轴方向；Row 横向 / Column 纵向）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutDirection {
    #[default]
    Row,
    Column,
}

/// 布局样式（纯 Rust 公共类型，D4 最小字段）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutStyle {
    /// 建议宽度（None = 由布局系统决定）。
    pub width: Option<f32>,
    /// 建议高度。
    pub height: Option<f32>,
    /// 是否占满剩余空间。
    pub grow: bool,
    /// 容器主轴方向（D23：Accordion 内部 Column=标题上/内容下；DemoRoot Row=组件横排）。
    pub direction: LayoutDirection,
    /// 容器四周 padding（D23 残留 P1-2：20pt 内容边距，macOS 惯例）。
    pub padding: f32,
}

#[allow(clippy::derivable_impls)] // 手写 Default（语义等价 derive，保留可读性）
impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            grow: false,
            direction: LayoutDirection::default(),
            padding: 0.0,
        }
    }
}

impl LayoutStyle {
    /// 构造固定尺寸布局样式。
    pub fn fixed(w: f32, h: f32) -> Self {
        Self {
            width: Some(w),
            height: Some(h),
            grow: false,
            direction: LayoutDirection::default(),
            padding: 0.0,
        }
    }

    /// 设置容器主轴方向。
    pub fn with_direction(mut self, d: LayoutDirection) -> Self {
        self.direction = d;
        self
    }

    /// 设置容器四周 padding。
    pub fn with_padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }
}

/// 把 `LayoutStyle` 映射为 Taffy Style（仅 layout feature 编译期启用）。
#[cfg(feature = "layout")]
pub fn to_taffy_style(style: &LayoutStyle) -> taffy::prelude::Style {
    use taffy::prelude::*;
    let mut s = Style::default();
    if let Some(w) = style.width {
        s.size.width = Dimension::Length(w);
    }
    if let Some(h) = style.height {
        s.size.height = Dimension::Length(h);
    }
    if style.grow {
        s.flex_grow = 1.0;
    }
    if style.padding > 0.0 {
        let p = LengthPercentage::Length(style.padding);
        s.padding = Rect {
            left: p,
            top: p,
            right: p,
            bottom: p,
        };
    }
    s
}
