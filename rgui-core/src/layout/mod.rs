//! 布局子模块（由 `rgui-layout` 迁入，greenfield §B.1 / §C.1 layout/）。
//!
//! ## 设计约束
//!
//! - 纯 Rust 逻辑布局（Taffy 纯 Rust，无平台/GPU）——并入 core 干净（契约 §2.4）。
//! - `LayoutEngine` 封装 Taffy；`mapping` 子模块做 `LayoutStyle → Taffy Style` 映射，
//!   **不把 Taffy 类型泄漏到公共 API**（§C.1 明确）。
//! - `state` 子模块**不持有** `LayoutResult`（那是 `rgui-render` 的渲染缓存职责）。
//!
//! D4：实现最小布局——给定容器尺寸与子节点建议尺寸，产出 `LayoutResult`。
//! taffy 经 `layout` feature 门控；功能由 `#[cfg(feature="layout")]` 提供。

pub mod mapping;

pub use mapping::LayoutStyle;

use crate::geometry::{Point, Rect, Size};

/// 布局结果（纯逻辑）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LayoutResult {
    /// 布局尺寸。
    pub size: Size,
    /// 布局位置（相对父容器原点）。
    pub position: Point,
    /// 布局矩形（origin + size）。
    pub rect: Rect,
}

impl LayoutResult {
    /// 构造布局结果。
    pub fn new(size: Size, position: Point) -> Self {
        Self {
            size,
            position,
            rect: Rect::new(position.x as f32, position.y as f32, size.width, size.height),
        }
    }
}

/// 布局节点（Taffy node 封装；对外不暴露 Taffy 类型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutNode(pub u64);

impl LayoutNode {
    /// 构造布局节点。
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// 布局引擎（Taffy 封装）。
///
/// D4 最小实现：`compute` 在 `#[cfg(feature="layout")]` 下用 Taffy，从子节点建议尺寸
/// 计算容器布局尺寸（纯 Rust 可测）。
#[derive(Debug, Default, Clone)]
pub struct LayoutEngine;

impl LayoutEngine {
    /// 构造布局引擎。
    pub fn new() -> Self {
        Self
    }

    /// 计算容器布局（Taffy）。结果尺寸 = 容器边界（子节点按 flex 排布）。
    #[cfg(feature = "layout")]
    pub fn compute(&self, container: Size, children: &[Size]) -> LayoutResult {
        use taffy::prelude::*;
        let mut taffy = TaffyTree::<()>::new();

        // 子节点：固定尺寸 leaf，顺序排布（flex 行）
        let mut child_ids = Vec::with_capacity(children.len());
        for size in children {
            let mut style = Style::default();
            style.size.width = Dimension::Length(size.width);
            style.size.height = Dimension::Length(size.height);
            let id = taffy.new_leaf(style).expect("new_leaf");
            child_ids.push(id);
        }

        let root_style = Style {
            size: Size {
                width: Dimension::Length(container.width),
                height: Dimension::Length(container.height),
            },
            ..Style::default()
        };

        // 建根节点：管理子节点（flex 容器）
        let root = match child_ids.as_slice() {
            [] => taffy.new_leaf(root_style).expect("new_leaf"),
            _ => taffy
                .new_with_children(root_style, &child_ids)
                .expect("new_with_children"),
        };

        taffy
            .compute_layout(
                root,
                taffy::geometry::Size {
                    width: taffy::AvailableSpace::MaxContent,
                    height: taffy::AvailableSpace::MaxContent,
                },
            )
            .expect("compute_layout");

        let layout = taffy.layout(root).expect("layout");
        let w = layout.size.width;
        let h = layout.size.height;
        let result_size = crate::geometry::Size::new(w, h);
        LayoutResult::new(result_size, Point::new(0, 0))
    }

    /// 计算容器布局，并对每个子节点返回其布局结果（位置 + 尺寸）。
    /// D6：from_view 用它获得子节点真实 bounds（布局真正作用于渲染）。
    #[cfg(feature = "layout")]
    pub fn compute_children(
        &self,
        container: Size,
        children: &[Size],
    ) -> Vec<LayoutResult> {
        use taffy::prelude::*;
        let mut taffy = TaffyTree::<()>::new();

        // 子节点：固定尺寸 leaf
        let mut child_ids = Vec::with_capacity(children.len());
        for size in children {
            let mut style = Style::default();
            style.size.width = Dimension::Length(size.width);
            style.size.height = Dimension::Length(size.height);
            let id = taffy.new_leaf(style).expect("new_leaf");
            child_ids.push(id);
        }

        let root_style = Style {
            size: Size {
                width: Dimension::Length(container.width),
                height: Dimension::Length(container.height),
            },
            // 主轴纵向排列（默认 flex column）
            ..Style::default()
        };

        let root = match child_ids.as_slice() {
            [] => taffy.new_leaf(root_style).expect("new_leaf"),
            _ => taffy
                .new_with_children(root_style, &child_ids)
                .expect("new_with_children"),
        };

        taffy
            .compute_layout(
                root,
                taffy::geometry::Size {
                    width: taffy::AvailableSpace::MaxContent,
                    height: taffy::AvailableSpace::MaxContent,
                },
            )
            .expect("compute_layout");

        // 读取每个子节点的位置 + 尺寸
        let mut results = Vec::with_capacity(children.len());
        for id in &child_ids {
            let l = taffy.layout(*id).expect("layout");
            results.push(LayoutResult::new(
                crate::geometry::Size::new(l.size.width, l.size.height),
                Point::new(l.location.x as i32, l.location.y as i32),
            ));
        }
        results
    }
}

// 布局计算在 `#[cfg(feature="layout")]` 下由 mapping/taffy 提供。
// 占位空实现保证非 layout feature 也能编译（core 保持可离线编译核心）。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_result_reconstructs_rect() {
        let r = LayoutResult::new(Size::new(80.0, 40.0), Point::new(10, 20));
        assert_eq!(r.size.width, 80.0);
        assert_eq!(r.rect.x, 10.0);
        assert_eq!(r.rect.y, 20.0);
        assert_eq!(r.rect.width, 80.0);
    }
}
