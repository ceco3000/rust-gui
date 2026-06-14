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
pub mod skyline;
pub mod texture;

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
pub use skyline::{Allocation, SkylineAllocator};
pub use texture::{TextureData, TextureFormat, TextureId};

#[cfg(feature = "skia-backend")]
pub use offscreen::OffscreenTestRunner;
pub use screenshot::{ScreenshotTolerance, delta_e, pixel_diff_ratio};
#[cfg(feature = "offscreen")]
pub use screenshot::{assert_screenshot_matches, assert_screenshot_matches_with_tolerance};
#[cfg(feature = "skia-backend")]
pub use skia::SkiaBackend;
