//! 场景构建——PaintOp → DrawCommand 转换 + SceneGraph 组装。
//!
//! 本模块提供将 `rgui_core::PaintOp`（组件 paint() 输出）
//! 转换为 `DrawCommand` 并组装为 `SceneGraph` 的桥接层。
//!
//! 定义源自 D3 §5（绘制阶段）和 D8 R19。

use rgui_core::Color;
use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::id::WidgetId;

use crate::scene::{DrawCommand, SceneGraph, SceneGraphBuilder};

// ============================================================================
// PaintOp → DrawCommand 转换
// ============================================================================

/// 将单个 `PaintOp` 转换为 `DrawCommand`（无文本渲染器时的回退路径）。
///
/// `PaintOp::DrawText` 会被转换为填充矩形占位符。
/// 如需真正的字形渲染，请使用
/// [`paint_op_to_draw_command_with_text`] 并传入 `TextRenderer`。
pub fn paint_op_to_draw_command(op: &PaintOp) -> DrawCommand {
    paint_op_to_draw_command_inner(op, None)
}

/// 将单个 `PaintOp` 转换为 `DrawCommand`，支持文本渲染。
///
/// 当 `text_renderer` 为 `Some` 且遇到 `PaintOp::DrawText` 时，
/// 会通过 `TextRenderer` 进行字形塑形和光栅化，生成真正的
/// `DrawCommand::DrawGlyphs` 指令。
pub fn paint_op_to_draw_command_with_text(
    op: &PaintOp,
    text_renderer: &crate::text_renderer::TextRenderer,
) -> DrawCommand {
    paint_op_to_draw_command_inner(op, Some(text_renderer))
}

/// 内部实现：根据可选的 TextRenderer 决定文本渲染策略。
fn paint_op_to_draw_command_inner(
    op: &PaintOp,
    text_renderer: Option<&crate::text_renderer::TextRenderer>,
) -> DrawCommand {
    match *op {
        PaintOp::FillRect {
            rect,
            color,
            radius,
        } => DrawCommand::FillRect {
            rect,
            color,
            radius,
        },
        PaintOp::DrawText {
            ref text,
            bounds,
            color,
            font_size,
        } => {
            if let Some(tr) = text_renderer {
                // 一次光栅化同时拿到渲染指令和度量数据（ascent/descent）
                let (mut commands, metrics) = tr.render_text(text, 0.0, 0.0, color, font_size);
                if commands.is_empty() {
                    return DrawCommand::FillRect {
                        rect: Rect::ZERO,
                        color: Color::TRANSPARENT,
                        radius: 0.0,
                    };
                }
                // 水平居中
                let baseline_x = bounds.origin.x as f32
                    + ((bounds.size.width as f32 - metrics.width) / 2.0).max(0.0);
                // 垂直：按 ascent 居中（视觉内容居中，忽略 descent 空白区）
                let baseline_y =
                    bounds.origin.y as f32 + (bounds.size.height as f32 + metrics.ascent) / 2.0;
                // 将预先计算的基线偏移应用到 glyph 位置
                let dx = baseline_x;
                let dy = baseline_y;
                let cmd = commands.remove(0);
                if let DrawCommand::DrawGlyphs {
                    texture_id,
                    mut glyphs,
                    font_size: fs,
                    color: c,
                } = cmd
                {
                    for g in &mut glyphs {
                        g.offset_x += dx;
                        g.offset_y += dy;
                    }
                    DrawCommand::DrawGlyphs {
                        texture_id,
                        glyphs,
                        font_size: fs,
                        color: c,
                    }
                } else {
                    cmd
                }
            } else {
                // 文本渲染失败时（如 CJK 无字体），产生不可见占位指令，
                // 避免 FillRect 遮盖同层其他元素
                DrawCommand::FillRect {
                    rect: Rect::ZERO,
                    color: Color::TRANSPARENT,
                    radius: 0.0,
                }
            }
        },
        PaintOp::DrawImage { rect } => {
            // 阶段 0：DrawImage 占位——VelloBackend 以洋红色半透明矩形渲染
            // 未来阶段：通过纹理注册机制使用实际 RGBA 像素数据
            DrawCommand::DrawImage {
                texture_id: crate::texture::TextureId(0),
                src: Rect::ZERO,
                dst: rect,
                blend_mode: crate::primitives::BlendMode::SrcOver,
            }
        },
    }
}

// ============================================================================
// 场景图层数据
// ============================================================================

/// 单个图层的绘制数据——从组件 paint() 收集到的结果。
#[derive(Debug, Clone)]
pub struct PaintLayerData {
    /// Widget 标识符。
    pub widget_id: WidgetId,
    /// Z 轴顺序（数值越大越靠前）。
    pub z_index: i32,
    /// Widget 在窗口中的边界矩形。
    pub bounds: Rect,
    /// 该 widget 的绘制操作列表。
    pub operations: Vec<PaintOp>,
    /// 是否标记为脏（需要重新编码到 Vello）。
    pub is_dirty: bool,
}

impl PaintLayerData {
    /// 创建新的图层数据。
    #[must_use]
    pub fn new(widget_id: WidgetId, z_index: i32, bounds: Rect, operations: Vec<PaintOp>) -> Self {
        Self {
            widget_id,
            z_index,
            bounds,
            operations,
            is_dirty: true,
        }
    }
}

// ============================================================================
// SceneGraph 构建
// ============================================================================

/// 从图层数据列表构建 `SceneGraph`。
///
/// 每个 `PaintLayerData` 对应一个 widget 的 paint() 输出。
/// 函数将 PaintOp 转换为 DrawCommand，按 z_index 排序，
/// 并生成最终的 SceneGraph。
///
/// `text_renderer` 可选——传入 `Some(&mut TextRenderer)` 可启用真正的
/// 字形渲染；传入 `None` 则 DrawText 会回退为填充矩形。
///
/// # 参数
///
/// - `layers`: 图层数据列表。
/// - `version`: 场景图版本号（通常为帧计数）。
/// - `text_renderer`: 可选的文本渲染器引用。
pub fn build_scene_from_paint_data(
    layers: &[PaintLayerData],
    version: u64,
    text_renderer: Option<&crate::text_renderer::TextRenderer>,
) -> SceneGraph {
    let mut builder = SceneGraphBuilder::new(version);

    for layer in layers {
        let mut commands = Vec::with_capacity(layer.operations.len());
        for op in &layer.operations {
            let cmd = paint_op_to_draw_command_inner(op, text_renderer);
            commands.push(cmd);
        }

        builder.build_layer(
            layer.widget_id,
            layer.z_index,
            layer.bounds,
            commands,
            layer.is_dirty,
        );
    }

    builder.finish()
}

/// 从单个 widget 的 paint 结果构建单图层 SceneGraph。
///
/// 便捷函数：适用于简单场景（如单个组件的离屏测试）。
pub fn build_single_layer_scene(
    widget_id: WidgetId,
    bounds: Rect,
    operations: &[PaintOp],
    version: u64,
) -> SceneGraph {
    let layer = PaintLayerData::new(widget_id, 0, bounds, operations.to_vec());
    build_scene_from_paint_data(&[layer], version, None)
}

// ============================================================================
// WidgetView 树遍历 → SceneGraph 构建
// ============================================================================

/// 遍历 WidgetView 树的回调：为每个 widget 提供 paint 函数。
///
/// 回调接收 widget 类型名、WidgetId、bounds，应返回该 widget 的 PaintOp 列表。
pub type PaintFn = Box<dyn Fn(&str, WidgetId, Rect) -> Vec<PaintOp> + Send + Sync>;

/// 从 WidgetView 树构建 SceneGraph。
///
/// 深度优先遍历 WidgetView 树，对每个节点调用 `paint_fn` 获取绘制操作，
/// 组装为图层并构建 SceneGraph。
///
/// # 参数
///
/// - `root`: 根 WidgetView（通常由 `ui!` 宏生成）。
/// - `root_bounds`: 根 widget 的窗口边界矩形（通常为窗口尺寸）。
/// - `paint_fn`: 为每个 widget 生成 PaintOp 的回调。
/// - `version`: 场景图版本号。
/// - `text_renderer`: 可选的文本渲染器。
pub fn build_scene_from_view<M: rgui_core::traits::AppMessage>(
    root: &rgui_core::view::WidgetView<M>,
    root_bounds: Rect,
    paint_fn: &PaintFn,
    version: u64,
    text_renderer: Option<&crate::text_renderer::TextRenderer>,
) -> SceneGraph {
    let mut builder = SceneGraphBuilder::new(version);
    let mut z_index: i32 = 0;

    walk_view_tree(
        root,
        root_bounds,
        paint_fn,
        &mut builder,
        &mut z_index,
        text_renderer,
    );

    builder.finish()
}

/// 深度优先遍历 WidgetView 树，收集绘制操作并构建图层。
fn walk_view_tree<M: rgui_core::traits::AppMessage>(
    view: &rgui_core::view::WidgetView<M>,
    bounds: Rect,
    paint_fn: &PaintFn,
    builder: &mut SceneGraphBuilder,
    z_index: &mut i32,
    text_renderer: Option<&crate::text_renderer::TextRenderer>,
) {
    let widget_id = view.id.unwrap_or_default();

    let ops = paint_fn(view.widget_type, widget_id, bounds);

    let mut commands = Vec::with_capacity(ops.len());
    for op in &ops {
        commands.push(paint_op_to_draw_command_inner(op, text_renderer));
    }

    builder.build_layer(widget_id, *z_index, bounds, commands, true);
    *z_index += 1;

    // 递归处理子节点——子节点 bounds 继承父节点 bounds
    for child in &view.children {
        walk_view_tree(child, bounds, paint_fn, builder, z_index, text_renderer);
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::Color;

    // --- PaintOp → DrawCommand ---

    #[test]
    fn convert_fill_rect() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let op = PaintOp::FillRect {
            rect,
            color: Color::RED,
            radius: 4.0,
        };
        let cmd = paint_op_to_draw_command(&op);
        assert!(matches!(
            cmd,
            DrawCommand::FillRect {
                rect: r,
                color: c,
                radius: 4.0,
            } if r == rect && c == Color::RED
        ));
    }

    #[test]
    fn convert_draw_text() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
        let op = PaintOp::DrawText {
            text: "Hello".into(),
            bounds,
            color: Color::BLACK,
            font_size: 14.0,
        };
        let cmd = paint_op_to_draw_command(&op);
        // 当前阶段文本渲染使用 FillRect 占位
        assert!(matches!(cmd, DrawCommand::FillRect { .. }));
    }

    // --- build_scene_from_paint_data ---

    #[test]
    fn build_scene_empty_layers() {
        let scene = build_scene_from_paint_data(&[], 0, None);
        assert!(scene.is_empty());
    }

    #[test]
    fn build_scene_single_layer() {
        let layers = vec![PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            Rect::new(0.0, 0.0, 100.0, 50.0),
            vec![PaintOp::FillRect {
                rect: Rect::new(0.0, 0.0, 100.0, 50.0),
                color: Color::BLUE,
                radius: 0.0,
            }],
        )];

        let scene = build_scene_from_paint_data(&layers, 42, None);
        assert_eq!(scene.layer_count(), 1);
        assert_eq!(scene.version, 42);
    }

    #[test]
    fn build_scene_multiple_layers_ordered_by_z() {
        let layer_back = PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            vec![],
        );
        let layer_front = PaintLayerData::new(
            WidgetId::from_u64(2),
            10,
            Rect::new(0.0, 0.0, 50.0, 50.0),
            vec![],
        );

        let scene = build_scene_from_paint_data(&[layer_front, layer_back], 0, None);
        assert_eq!(scene.layer_count(), 2);
        // SceneGraphBuilder::finish() 按 z_index 排序
        assert_eq!(scene.layers[0].z_index, 0);
        assert_eq!(scene.layers[1].z_index, 10);
    }

    // --- build_single_layer_scene ---

    #[test]
    fn build_single_layer() {
        let ops = vec![PaintOp::FillRect {
            rect: Rect::new(0.0, 0.0, 50.0, 50.0),
            color: Color::GREEN,
            radius: 2.0,
        }];
        let scene = build_single_layer_scene(
            WidgetId::from_u64(1),
            Rect::new(0.0, 0.0, 50.0, 50.0),
            &ops,
            1,
        );
        assert_eq!(scene.layer_count(), 1);
        assert_eq!(scene.layers[0].widget_id, WidgetId::from_u64(1));
        assert_eq!(scene.layers[0].commands.len(), 1);
    }

    // --- PaintLayerData ---

    #[test]
    fn paint_layer_data_new_is_dirty() {
        let layer = PaintLayerData::new(WidgetId::from_u64(1), 0, Rect::ZERO, vec![]);
        assert!(layer.is_dirty);
        assert_eq!(layer.widget_id, WidgetId::from_u64(1));
    }
}
