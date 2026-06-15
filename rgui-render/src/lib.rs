//! # rgui-render
//!
//! rgui 渲染管线——SceneGraph、Vello/Skia 后端、字形 Atlas。

pub mod backend;
pub mod dirty;
pub mod factory;
pub mod focus_indicator;
#[cfg(feature = "vello-backend")]
pub mod font;
pub mod glyph;
pub mod primitives;
pub mod scene;
pub mod scene_build;
pub mod skyline;
#[cfg(feature = "vello-backend")]
pub mod text;
#[cfg(feature = "vello-backend")]
pub mod text_renderer;
pub mod texture;

#[cfg(feature = "vello-backend")]
pub mod vello;

#[cfg(feature = "skia-backend")]
pub mod offscreen;
pub mod screenshot;
#[cfg(feature = "skia-backend")]
pub mod skia;

pub use backend::{RenderBackend, RenderError, RenderParams};
pub use dirty::DirtyRegionTracker;
pub use factory::{BackendType, RenderBackendFactory};
pub use focus_indicator::FocusIndicator;
pub use glyph::GlyphAtlas;
pub use primitives::{
    BlendMode, FillRule, GlyphData, GradientStop, ImageRepeat, LineCap, LineJoin, Paint,
    PathCommand, PathData, Stroke, Transform,
};
pub use scene::{ClipRegion, DrawCommand, SceneGraph, SceneGraphBuilder, SceneLayer, TextureRef};
pub use scene_build::{
    PaintFn, PaintLayerData, build_scene_from_paint_data, build_scene_from_view,
    build_single_layer_scene, paint_op_to_draw_command, paint_op_to_draw_command_with_text,
};
pub use skyline::{Allocation, SkylineAllocator};
#[cfg(feature = "vello-backend")]
pub use text_renderer::TextRenderer;
pub use texture::{TextureData, TextureFormat, TextureId};

#[cfg(feature = "skia-backend")]
pub use offscreen::OffscreenTestRunner;
pub use screenshot::{ScreenshotTolerance, delta_e, pixel_diff_ratio};
#[cfg(feature = "offscreen")]
pub use screenshot::{assert_screenshot_matches, assert_screenshot_matches_with_tolerance};
#[cfg(feature = "skia-backend")]
pub use skia::SkiaBackend;
#[cfg(feature = "vello-backend")]
pub use vello::{VelloBackend, encode_scene_to_vello};
