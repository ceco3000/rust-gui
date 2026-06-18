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

use rgui_layout::{LayoutEngine, LayoutNode};

use crate::primitives::Transform;
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
    /// 裁剪矩形（可选）。
    ///
    /// 当设置时，该图层的绘制命令会被 `PushClip`/`PopClip` 包裹，
    /// 裁剪到该矩形。用于 ScrollView 等需要裁剪子内容的容器。
    pub clip_rect: Option<Rect>,
    /// 滚动偏移量（可选）。
    ///
    /// 当设置时，该图层的绘制命令会被 `PushTransform(translate)`/`PopTransform`
    /// 包裹。坐标 (dx, dy) 表示平移量——通常为 `(-scroll_x, -scroll_y)`。
    /// 用于 ScrollView 对子内容的滚动偏移。
    pub scroll_offset: Option<(f32, f32)>,
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
            clip_rect: None,
            scroll_offset: None,
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
        let mut commands = Vec::with_capacity(
            layer.operations.len()
                + layer.clip_rect.is_some() as usize * 2
                + layer.scroll_offset.is_some() as usize * 2,
        );

        // 先 PushClip（如果有）
        if let Some(clip) = layer.clip_rect {
            commands.push(DrawCommand::PushClip { rect: clip });
        }
        // 再 PushTransform（如果有）——注意平移方向为子内容偏移
        if let Some((dx, dy)) = layer.scroll_offset {
            commands.push(DrawCommand::PushTransform {
                transform: Transform::translate(dx, dy),
            });
        }

        for op in &layer.operations {
            let cmd = paint_op_to_draw_command_inner(op, text_renderer);
            commands.push(cmd);
        }

        // 后 PopTransform（如果有）——与 Push 顺序相反
        if layer.scroll_offset.is_some() {
            commands.push(DrawCommand::PopTransform);
        }
        // 后 PopClip（如果有）
        if layer.clip_rect.is_some() {
            commands.push(DrawCommand::PopClip);
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
/// - `root`: 根 WidgetView（通常由 `html!` 宏生成）。
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
// WidgetView 树 → Taffy 布局计算
// ============================================================================

/// 计算 WidgetView 树的 Taffy 布局。
///
/// 为树中每个节点分配唯一的 `WidgetId`，为每个节点创建 Taffy layout node，
/// 并从 `WidgetView.props` 中提取 CSS 布局属性映射到 Taffy `Style`。
///
/// 返回的 `LayoutEngine` 中缓存了所有节点的计算后位置和尺寸，
/// 可通过 `engine.get_layout(widget_id)` 查询。
///
/// # 参数
///
/// - `view`：可变引用到 WidgetView 树（将被修改：为每个节点设置 `id` 字段）。
/// - `available`：可用空间尺寸（通常为窗口像素尺寸）。
///
/// # 实现说明
///
/// - DFS 后序遍历：先处理子节点，再创建父节点（Taffy 要求）。
/// - 容器组件（Container/Row/Column 等）从 props 中提取完整 CSS 布局属性。
/// - 叶子组件（Button/Label 等）使用默认 Taffy Style（无特殊布局属性）。
pub fn compute_view_layout<M: rgui_core::traits::AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    available: rgui_core::geometry::Size,
) -> LayoutEngine {
    let mut engine = LayoutEngine::new();
    let root = build_layout_tree(view, &mut engine);
    engine.compute_layout(root, available);
    engine
}

/// DFS 后序遍历 WidgetView 树，为每个节点创建 Taffy layout node。
fn build_layout_tree<M: rgui_core::traits::AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    engine: &mut LayoutEngine,
) -> LayoutNode {
    // 后序：先递归子节点
    let child_nodes: Vec<LayoutNode> = view
        .children
        .iter_mut()
        .map(|child| build_layout_tree(child, engine))
        .collect();

    // 从 props 提取 Taffy Style
    let style = extract_taffy_style_from_props(&view.props);

    // 分配唯一 WidgetId
    let widget_id = WidgetId::new();
    view.id = Some(widget_id);

    // 创建 Taffy 节点
    engine.create_node(widget_id, style, &child_nodes)
}

/// 从 `WidgetView.props` 中提取 CSS 布局属性并转换为 Taffy `Style`。
///
/// 支持的布局属性：
/// - `display`、`flex-direction`、`justify-content`、`align-items`（字符串）。
/// - `width`、`height`、`gap`、`padding`、`margin`（数值）。
///
/// 未识别的属性或非布局属性（`label`、`text`、`checked` 等）被忽略。
fn extract_taffy_style_from_props(
    props: &std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
) -> taffy::Style {
    use rgui_core::view::PropValue;

    let get_str = |key: &str| -> Option<&str> {
        match props.get(key) {
            Some(PropValue::Str(s)) => Some(s),
            _ => None,
        }
    };

    let get_f32 = |key: &str| -> Option<f32> {
        match props.get(key) {
            Some(PropValue::Float(f)) => Some(f.0 as f32),
            Some(PropValue::Int(i)) => Some(*i as f32),
            _ => None,
        }
    };

    rgui_layout::mapping::to_taffy_style(
        get_str("display"),
        get_f32("width"),
        get_f32("height"),
        get_str("flex-direction"),
        get_str("justify-content"),
        get_str("align-items"),
        get_f32("gap"),
        get_f32("padding"),
        get_f32("margin"),
    )
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::Color;
    use rgui_core::geometry::Size;

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

    #[test]
    fn paint_layer_data_clip_rect_defaults_none() {
        let layer = PaintLayerData::new(WidgetId::from_u64(1), 0, Rect::ZERO, vec![]);
        assert!(layer.clip_rect.is_none());
        assert!(layer.scroll_offset.is_none());
    }

    #[test]
    fn build_scene_with_clip_rect() {
        let mut layer = PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            Rect::new(0.0, 0.0, 400.0, 300.0),
            vec![PaintOp::FillRect {
                rect: Rect::new(0.0, 0.0, 500.0, 500.0),
                color: Color::RED,
                radius: 0.0,
            }],
        );
        layer.clip_rect = Some(Rect::new(0.0, 0.0, 300.0, 200.0));

        let scene = build_scene_from_paint_data(&[layer], 1, None);
        assert_eq!(scene.layer_count(), 1);
        let cmds = &scene.layers[0].commands;
        // PushClip + FillRect + PopClip = 3 commands
        assert_eq!(cmds.len(), 3);
        assert!(matches!(cmds[0], DrawCommand::PushClip { .. }));
        assert!(matches!(cmds[1], DrawCommand::FillRect { .. }));
        assert!(matches!(cmds[2], DrawCommand::PopClip));
    }

    #[test]
    fn build_scene_with_scroll_offset() {
        let mut layer = PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            Rect::new(0.0, 0.0, 400.0, 300.0),
            vec![PaintOp::FillRect {
                rect: Rect::new(0.0, 0.0, 500.0, 1000.0),
                color: Color::BLUE,
                radius: 0.0,
            }],
        );
        layer.scroll_offset = Some((-10.0, -50.0));

        let scene = build_scene_from_paint_data(&[layer], 1, None);
        assert_eq!(scene.layer_count(), 1);
        let cmds = &scene.layers[0].commands;
        // PushTransform + FillRect + PopTransform = 3 commands
        assert_eq!(cmds.len(), 3);
        assert!(matches!(cmds[0], DrawCommand::PushTransform { .. }));
        assert!(matches!(cmds[1], DrawCommand::FillRect { .. }));
        assert!(matches!(cmds[2], DrawCommand::PopTransform));
    }

    #[test]
    fn build_scene_with_both_clip_and_offset() {
        let mut layer = PaintLayerData::new(
            WidgetId::from_u64(1),
            0,
            Rect::new(0.0, 0.0, 400.0, 300.0),
            vec![PaintOp::FillRect {
                rect: Rect::new(0.0, 0.0, 500.0, 1000.0),
                color: Color::GREEN,
                radius: 0.0,
            }],
        );
        layer.clip_rect = Some(Rect::new(0.0, 0.0, 300.0, 200.0));
        layer.scroll_offset = Some((-10.0, -50.0));

        let scene = build_scene_from_paint_data(&[layer], 1, None);
        assert_eq!(scene.layer_count(), 1);
        let cmds = &scene.layers[0].commands;
        // PushClip + PushTransform + FillRect + PopTransform + PopClip = 5 commands
        assert_eq!(cmds.len(), 5);
        assert!(matches!(cmds[0], DrawCommand::PushClip { .. }));
        assert!(matches!(cmds[1], DrawCommand::PushTransform { .. }));
        assert!(matches!(cmds[2], DrawCommand::FillRect { .. }));
        assert!(matches!(cmds[3], DrawCommand::PopTransform));
        assert!(matches!(cmds[4], DrawCommand::PopClip));
    }

    // --- compute_view_layout ---

    /// 测试用消息类型。
    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Dummy,
    }

    impl rgui_core::traits::AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            match self {
                Self::Dummy => "dummy",
            }
        }
    }

    /// 辅助函数：创建带 widget_type 和子节点的 WidgetView。
    fn make_view(
        widget_type: &'static str,
        children: Vec<rgui_core::view::WidgetView<TestMsg>>,
    ) -> rgui_core::view::WidgetView<TestMsg> {
        rgui_core::view::WidgetView::new(widget_type).children(children)
    }

    /// 辅助函数：创建带 text 属性的 Label WidgetView。
    fn make_label(text: &str) -> rgui_core::view::WidgetView<TestMsg> {
        rgui_core::view::WidgetView::new("Label")
            .prop("text", rgui_core::view::PropValue::str(text))
    }

    /// 辅助函数：创建带 label 属性的 Button WidgetView。
    fn make_button(label: &str) -> rgui_core::view::WidgetView<TestMsg> {
        rgui_core::view::WidgetView::new("Button")
            .prop("label", rgui_core::view::PropValue::str(label))
    }

    #[test]
    fn compute_view_layout_assigns_unique_ids() {
        let mut view = make_view("Column", vec![make_label("Hello"), make_button("Click")]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0));

        // 根节点有 ID
        assert!(view.id.is_some());
        let root_id = view.id.unwrap();
        assert_ne!(root_id, WidgetId::default());

        // 子节点也有 ID，且与根节点不同
        for child in &view.children {
            assert!(child.id.is_some());
            assert_ne!(child.id.unwrap(), root_id);
        }

        // LayoutEngine 中有布局结果
        assert!(engine.get_layout(root_id).is_some());
    }

    #[test]
    fn compute_view_layout_column_children_spaced() {
        let mut view = make_view("Column", vec![make_label("Line 1"), make_label("Line 2")]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0));

        let root_id = view.id.unwrap();
        let root_layout = engine.get_layout(root_id).unwrap();
        // 根 Column 的位置应为原点
        assert!((root_layout.result.position.x - 0.0).abs() < 1.0);
        assert!((root_layout.result.position.y - 0.0).abs() < 1.0);
        // 子节点也被布局（有结果）
        for child in &view.children {
            assert!(engine.get_layout(child.id.unwrap()).is_some());
        }
    }

    #[test]
    fn compute_view_layout_reassigns_ids_on_each_call() {
        let mut view1 = make_view("Row", vec![make_button("A")]);
        let mut view2 = make_view("Row", vec![make_button("B")]);

        let _e1 = compute_view_layout(&mut view1, Size::new(200.0, 100.0));
        let _e2 = compute_view_layout(&mut view2, Size::new(200.0, 100.0));

        // 不同调用分配不同 ID
        assert_ne!(view1.id, view2.id);
        assert_ne!(view1.children[0].id, view2.children[0].id);
    }

    #[test]
    fn compute_view_layout_empty_view() {
        let mut view = make_view("Container", vec![]);

        let engine = compute_view_layout(&mut view, Size::new(800.0, 600.0));

        assert!(view.id.is_some());
        let layout = engine.get_layout(view.id.unwrap()).unwrap();
        // 空容器返回有效的布局结果（位置在原点）
        assert!(layout.result.position.x >= 0.0);
        assert!(layout.result.position.y >= 0.0);
    }
}
