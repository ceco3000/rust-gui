//! # rgui-render
//!
//! rgui 渲染管线——SceneGraph、Vello/Skia 后端、字形 Atlas。

pub mod scene;
pub mod primitives;
pub mod texture;
pub mod backend;
pub mod glyph;

pub use scene::{SceneGraph, SceneLayer, ClipRegion, DrawCommand, TextureRef, SceneGraphBuilder};
pub use primitives::{
    BlendMode, FillRule, GlyphData, GradientStop, ImageRepeat, LineCap, LineJoin, Paint,
    PathCommand, PathData, Stroke, Transform,
};
pub use texture::{TextureData, TextureFormat, TextureId};
pub use backend::{RenderBackend, RenderError, RenderParams};
pub use glyph::GlyphAtlas;
