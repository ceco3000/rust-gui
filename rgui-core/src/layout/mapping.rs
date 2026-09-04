//! 布局样式映射：`LayoutStyle → Taffy Style` 封装（greenfield §C.1 layout/mapping.rs）。
//!
//! **不把 Taffy 类型泄漏到公共 API**：`LayoutStyle` 为纯 Rust 公共类型，`to_taffy_style`
//! 仅在 `#[cfg(feature="layout")]` 下实现（依赖 taffy）。Taffy 是纯 Rust，无平台/GPU。

/// 布局样式（纯 Rust 公共类型，D4 最小字段）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutStyle {
    /// 建议宽度（None = 由布局系统决定）。
    pub width: Option<f32>,
    /// 建议高度。
    pub height: Option<f32>,
    /// 是否占满剩余空间。
    pub grow: bool,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            grow: false,
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
        }
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
    s
}
