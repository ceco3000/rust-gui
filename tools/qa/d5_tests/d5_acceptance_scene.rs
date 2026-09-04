//! D5 验收测试 · SceneGraph 从 WidgetView 转换（A 层纯逻辑，无 GPU）
//!
//! 注入：cp tools/qa/d5_tests/d5_acceptance_scene.rs rgui-render/tests/
//! 运行：cargo test -p rgui-render --test d5_acceptance_scene
//!
//! ⚠️ 契约锁定：`DrawCmd`/`SceneGraph::from_view` 为 dev D5 实现后应暴露的 API。
//! 当前 SceneGraph 为占位（{root:Option<SceneNode>}），故 from_view 相关用例待实现后 PASS；
//! 为防骨架整体编译失败，先把**当前可编译**的占位断言与**契约锁定**分开（后者待 dev 落地后启用）。

use rgui_core::view::{WidgetView, PropValue, Color};
use rgui_core::geometry::{Point, Rect, Size};
use rgui_render::scene_graph::{SceneGraph, SceneNode};
use rgui_render::glyph::{GlyphKey, GlyphCacheEntry};
use rgui_render::PathTessellation;

// ============ 当前可编译（占位契约，验证骨架自身正确） ============

#[test]
fn sg0_scenegraph_constructible_no_panic() {
    let sg = SceneGraph::new();
    assert!(sg.root.is_none());
}

#[test]
fn sg0b_scenenode_default() {
    let n = SceneNode::default();
    assert!(n.name.is_empty());
    assert!(n.children.is_empty());
}

#[test]
fn sg0c_render_layout_cache_types_available() {
    // RenderLayoutCache 所需 GPU 资源类型可构造（契约 §2 方案 A 移入 render）
    let _gk = GlyphKey { glyph_id: 1, font_id: 2 };
    let _ge = GlyphCacheEntry(0);
    let _pt = PathTessellation::default();
    let _ = _gk; let _ = _ge; let _ = _pt;
}

// ============ 契约锁定（待 dev 实现 from_view/DrawCmd 后启用，见 D5 清单 §2） ============

// 注：以下用例在 dev 尚未暴露 `SceneGraph::from_view`/`DrawCmd` 时不可编译，
// 故未列入本骨架。待 dev 实现后按 D5 清单 §2.1（SG1-SG7）补充：
//   - SG1 空视图 → 空 draw
//   - SG2 单矩形 → 1 条 Rect(color) 指令，颜色匹配
//   - SG3 文本 → 1 条 Text(text) 指令
//   - SG4 嵌套 children → parent+children 顺序 draw
//   - SG5 PropValue 类型映射
//   - SG6 rect/LayoutResult 应用
//   - SG7 默认 color 兜底

/// 契约探测：确认 from_view 存在（编译期守卫；dev 实现后自动编译通过）。
#[allow(dead_code)]
fn _contract_probe(v: &WidgetView<()>) {
    // let sg = SceneGraph::from_view(v);  // ← dev 实现后解除注释
    // let _ = &sg.draws;                   // ← dev 实现后补充
}
