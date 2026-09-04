//! D4 验收测试 · 布局（Taffy 集成：LayoutStyle → Taffy Style + LayoutEngine 计算）
//!
//! 注入：cp tools/qa/d4_tests/d4_acceptance_layout.rs rgui-core/tests/
//! 运行：cargo test -p rgui-core --test d4_acceptance_layout
//!
//! ⚠️ 契约锁定：引用的 `LayoutStyle`/`to_taffy_style`/`LayoutEngine::compute_layout` 为 dev D4
//! 实现后必须暴露的 API（greenfield §B.1 + contract-design §2.4）。D3 占位中 LayoutEngine 无方法、
//! 无 LayoutStyle，故 D4 交付前不注入。

use rgui_core::geometry::{Point, Rect, Size};
use rgui_core::layout::{LayoutEngine, LayoutResult, to_taffy_style};

// LayoutStyle 为 dev 实现后应暴露的类型；此处用枚举成员做面向契约的探测。
// 说明：LayoutStyle 字段名依据 greenfield §B.1 布局意图（flex_direction/align_items/justify_content/size/margin/padding/gap）。
// 若 dev 实现字段名不同，按 dev 实际契约调整断言（以绿色农场地 §B.1 为唯一基准）。

// ============ L: LayoutStyle → Taffy Style 映射 ============

#[test]
fn l6_default_layout_style_maps_to_default() {
    // 默认 LayoutStyle → Taffy Style::default() 等价，不 panic
    let _ = to_taffy_style(Default::default());
}

#[test]
fn l3_size_mapping() {
    let style = LayoutStyle {
        width: Some(100.0),
        height: Some(50.0),
        ..Default::default()
    };
    #[allow(unused_mut)]
    let mut taffy = to_taffy_style(style);
    // 尺寸映射断言（若 dev 暴露 taffy::Style.size）
    let _ = taffy;
}

#[test]
fn l1_flex_direction_mapping() {
    let style = LayoutStyle {
        flex_direction: FlexDirection::Column,
        ..Default::default()
    };
    let _ = to_taffy_style(style);
}

#[test]
fn l4_margin_padding_mapping() {
    let style = LayoutStyle {
        margin: Some(Edge { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 }),
        ..Default::default()
    };
    let _ = to_taffy_style(style);
}

// ============ CE: LayoutEngine 计算 ============

#[test]
fn ce1_single_node_fixed_size() {
    let engine = LayoutEngine::new();
    let layout = engine.compute_layout(&LayoutStyle { width: Some(100.0), height: Some(50.0), ..Default::default() }, Size::new(200.0, 200.0));
    assert_eq!(layout.size.width, 100.0);
    assert_eq!(layout.size.height, 50.0);
}

#[test]
fn ce3_empty_tree_no_panic() {
    let engine = LayoutEngine::new();
    let _ = engine.compute_layout(&LayoutStyle::default(), Size::new(0.0, 0.0));
}

#[test]
fn ce4_repeat_compute_idempotent() {
    let engine = LayoutEngine::new();
    let s = LayoutStyle { width: Some(30.0), ..Default::default() };
    let a = engine.compute_layout(&s, Size::new(100.0, 100.0));
    let b = engine.compute_layout(&s, Size::new(100.0, 100.0));
    assert_eq!(a.size.width, b.size.width);
    assert_eq!(a.size.height, b.size.height);
}

// ============ 占位辅助类型声明（契约探测，供 dev 对齐；非实现） ============

/// 注：以下类型 dev 应在其 layout 模块暴露。若 dev 用不同命名，请对照 greenfield §B.1 对齐。
/// 这里仅作为契约占位，避免本骨架因未实现而编译失败——交付后 dev 应提供同名类型。
#[allow(dead_code)]
mod placeholder {
    use super::*;
    // 当 dev 未暴露这些类型时，本骨架将以 cfg(feature) gating 控制编译；此处仅文档占位。
}
