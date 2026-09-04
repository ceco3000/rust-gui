//! 布局引擎 Taffy 集成测试（TDD RED 起点，feature="layout"）。
//!
//! 目标：`LayoutEngine::compute(container: Size, children: &[Size]) -> LayoutResult`
//! 用 Taffy 从子节点建议尺寸计算容器布局（纯 Rust 可测）。

#![cfg(all(test, feature = "layout"))]

use rgui_core::layout::{LayoutEngine, LayoutResult};
use rgui_core::geometry::Size;

#[test]
fn compute_single_child_lays_out() {
    let engine = LayoutEngine::new();
    let container = Size::new(100.0, 100.0);
    let children = vec![Size::new(50.0, 25.0)];
    let result: LayoutResult = engine.compute(container, &children);
    // 子节点应被布局在容器内，且不超过容器
    assert!(result.size.width <= container.width + f32::EPSILON);
    assert!(result.size.height <= container.height + f32::EPSILON);
    assert!(result.size.width >= 0.0);
    assert!(result.size.height >= 0.0);
}

#[test]
fn compute_multiple_children_no_panic() {
    let engine = LayoutEngine::new();
    let container = Size::new(200.0, 200.0);
    let children = vec![Size::new(80.0, 40.0), Size::new(80.0, 40.0)];
    let result = engine.compute(container, &children);
    let _ = result;
}
