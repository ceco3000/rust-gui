//! # rgui-render
//!
//! rgui 渲染引擎——**单一 vello 后端**（契约 §1.3 / §5）。
//!
//! ## 设计约束
//!
//! - 重型 GPU 隔离：wgpu/vello/cosmic-text/fontdb/skrifa 均隔离在此 crate。
//! - 删除 skia 后端与相关 feature（契约 §5「单一 vello」）。
//! - `glyph`/`path_tessellation`（GPU 资源类型）原生位于此 crate，并从 state 剥离后
//!   由 `rgui-render` 持有 `RenderLayoutCache`（契约 §2 方案 A）。
//! - 可依赖 `rgui-core`；反向禁止。
//!
//! D3 阶段 0：仅建立模块骨架 + 类型占位，不引入 GPU 依赖。

pub mod glyph;
pub mod path_tessellation;
pub mod scene_graph;
pub mod text;
#[cfg(feature = "vello-backend")]
pub mod vello;

// GPU 资源类型契约导出（实现阶段补全真实定义；D3 阶段 0 为占位签名）
pub use glyph::{GlyphAtlas, GlyphCacheEntry, GlyphKey};
pub use path_tessellation::PathTessellation;
pub use text::TextShaper;
pub use scene_graph::{DrawCmd, SceneGraph};
#[cfg(feature = "vello-backend")]
pub use vello::{RenderBackend, VelloBackend};

// D8：surface 类型别名（示例/ facade 经此引用，避免直接 `wgpu::` 前缀）
#[cfg(feature = "vello-backend")]
pub use wgpu::Surface as GpuSurface;

// 保持对 rgui-core 依赖有效（渲染层向下依赖逻辑层，合法）
#[allow(dead_code)]
fn _marks_core_dep() {
    let _ = rgui_core::geometry::Point::new(0, 0);
}

/// 渲染布局缓存——每挂载组件一份，生命周期 = 组件挂载→卸载。
/// 由 `rgui-state` 剥离至此（契约 §2 方案 A 定稿位置）。
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct RenderLayoutCache {
    /// 逻辑布局结果（引用 core::layout）。
    pub layout: Option<rgui_core::layout::LayoutResult>,
    /// GPU 字形缓存（key=GlyphKey，v=GlyphCacheEntry）。
    pub glyph_cache: std::collections::HashMap<GlyphKey, GlyphCacheEntry>,
    /// 路径细分。
    pub path_tessellation: Option<PathTessellation>,
    /// 上次绘制颜色。
    pub last_paint_color: Option<rgui_core::view::Color>,
}
