//! 场景图——SceneGraph、SceneLayer、DrawCommand、SceneGraphBuilder。
//!
//! 定义源自 D3 §3。

use crate::primitives::{BlendMode, GlyphData, Paint, PathData, Stroke, Transform};
use crate::texture::TextureId;
use rgui_core::Color;
use rgui_core::geometry::Rect;
use rgui_core::id::WidgetId;
use std::fmt;

// ============================================================================
// SceneGraph
// ============================================================================

/// 场景图：Widget 树 → 绘制指令的中间表示（D3 §3.1）。
#[derive(Clone)]
pub struct SceneGraph {
    pub layers: Vec<SceneLayer>,
    pub dirty_layers: Vec<usize>,
    pub clip_regions: Vec<ClipRegion>,
    pub texture_refs: Vec<TextureRef>,
    pub version: u64,
}

impl SceneGraph {
    #[must_use]
    pub fn new(version: u64) -> Self {
        Self {
            layers: Vec::new(),
            dirty_layers: Vec::new(),
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version,
        }
    }

    #[must_use]
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

impl fmt::Debug for SceneGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SceneGraph")
            .field("layers", &self.layers.len())
            .field("dirty", &self.dirty_layers.len())
            .field("version", &self.version)
            .finish()
    }
}

// ============================================================================
// SceneLayer
// ============================================================================

/// 单个绘制层（D3 §3.1）。
#[derive(Clone, Debug)]
pub struct SceneLayer {
    /// z 轴顺序（值越大越靠前）。
    pub z_index: i32,
    /// 层的包围矩形（窗口坐标）。
    pub bounds: Rect,
    /// 绘制指令列表。
    pub commands: Vec<DrawCommand>,
    /// 关联的 widget ID。
    pub widget_id: WidgetId,
    /// 透明度（0.0-1.0）。
    pub opacity: f32,
    /// 变换矩阵。
    pub transform: Option<Transform>,
}

impl SceneLayer {
    #[must_use]
    pub fn new(widget_id: WidgetId, z_index: i32, bounds: Rect) -> Self {
        Self {
            z_index,
            bounds,
            commands: Vec::new(),
            widget_id,
            opacity: 1.0,
            transform: None,
        }
    }

    pub fn push(&mut self, cmd: DrawCommand) {
        self.commands.push(cmd);
    }
}

// ============================================================================
// DrawCommand
// ============================================================================

/// 绘制指令枚举（D3 §3.1，D0 不变式 4：具体枚举，无 `Box<dyn>`）。
#[derive(Clone, Debug)]
pub enum DrawCommand {
    FillRect {
        rect: Rect,
        color: Color,
        radius: f32,
    },
    FillPath {
        path: PathData,
        paint: Paint,
    },
    StrokePath {
        path: PathData,
        stroke: Stroke,
        paint: Paint,
    },
    DrawGlyphs {
        texture_id: TextureId,
        glyphs: Vec<GlyphData>,
        font_size: f32,
        color: Color,
    },
    DrawImage {
        texture_id: TextureId,
        src: Rect,
        dst: Rect,
        blend_mode: BlendMode,
    },
    PushClip {
        rect: Rect,
    },
    PopClip,
    PushTransform {
        transform: Transform,
    },
    PopTransform,
    PushOpacity {
        opacity: f32,
    },
    PopOpacity,
}

// ============================================================================
// 辅助类型
// ============================================================================

/// 裁剪区域（D3 §3.1）。
#[derive(Clone, Debug)]
pub struct ClipRegion {
    pub rect: Rect,
    pub radius: f32,
}

/// 纹理引用（D3 §3.1）。
#[derive(Clone, Debug)]
pub struct TextureRef {
    pub texture_id: TextureId,
    pub width: u32,
    pub height: u32,
    pub format: crate::texture::TextureFormat,
}

// ============================================================================
// SceneGraphBuilder
// ============================================================================

/// Scene Graph 构建器（D3 §3.3）。
pub struct SceneGraphBuilder {
    layers: Vec<SceneLayer>,
    dirty_layers: Vec<usize>,
    clip_regions: Vec<ClipRegion>,
    texture_refs: Vec<TextureRef>,
    version: u64,
}

impl SceneGraphBuilder {
    #[must_use]
    pub fn new(version: u64) -> Self {
        Self {
            layers: Vec::new(),
            dirty_layers: Vec::new(),
            clip_regions: Vec::new(),
            texture_refs: Vec::new(),
            version,
        }
    }

    pub fn add_clip_region(&mut self, region: ClipRegion) {
        self.clip_regions.push(region);
    }

    pub fn add_texture_ref(&mut self, tex_ref: TextureRef) {
        self.texture_refs.push(tex_ref);
    }

    /// 构建一个 widget 的绘制层。
    pub fn build_layer(
        &mut self,
        widget_id: WidgetId,
        z_index: i32,
        bounds: Rect,
        commands: Vec<DrawCommand>,
        is_dirty: bool,
    ) {
        let layer_index = self.layers.len();
        self.layers.push(SceneLayer {
            z_index,
            bounds,
            commands,
            widget_id,
            opacity: 1.0,
            transform: None, // 坐标偏移已在 walk_view_tree 中通过 offset_draw_command 应用
        });
        if is_dirty {
            self.dirty_layers.push(layer_index);
        }
    }

    /// 返回当前已构建的图层列表（只读）。
    #[must_use]
    pub fn layers(&self) -> &[SceneLayer] {
        &self.layers
    }

    /// 完成构建，产出 SceneGraph。
    #[must_use]
    pub fn finish(mut self) -> SceneGraph {
        self.layers.sort_by_key(|l| l.z_index);
        SceneGraph {
            layers: self.layers,
            dirty_layers: self.dirty_layers,
            clip_regions: self.clip_regions,
            texture_refs: self.texture_refs,
            version: self.version,
        }
    }
}

impl fmt::Debug for SceneGraphBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SceneGraphBuilder")
            .field("layers", &self.layers.len())
            .finish()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_graph_new() {
        let sg = SceneGraph::new(0);
        assert_eq!(sg.version, 0);
        assert!(sg.is_empty());
    }

    #[test]
    fn scene_layer_push_command() {
        let widget_id = WidgetId::new();
        let mut layer = SceneLayer::new(widget_id, 0, Rect::ZERO);
        layer.push(DrawCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            color: Color::RED,
            radius: 4.0,
        });
        assert_eq!(layer.commands.len(), 1);
    }

    #[test]
    fn builder_build_layer() {
        let mut builder = SceneGraphBuilder::new(1);
        let widget_id = WidgetId::new();
        builder.build_layer(
            widget_id,
            0,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Vec::new(),
            true,
        );
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 1);
        assert_eq!(sg.dirty_layers.len(), 1);
    }

    #[test]
    fn builder_layers_sorted_by_z() {
        let mut builder = SceneGraphBuilder::new(1);
        let id1 = WidgetId::new();
        let id2 = WidgetId::new();
        builder.build_layer(id2, 10, Rect::ZERO, Vec::new(), false);
        builder.build_layer(id1, 1, Rect::ZERO, Vec::new(), false);
        let sg = builder.finish();
        assert_eq!(sg.layers[0].z_index, 1);
        assert_eq!(sg.layers[1].z_index, 10);
    }
}
