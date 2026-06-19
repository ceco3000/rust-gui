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
use crate::text_renderer::TextRenderer;

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
/// 回调接收 `&WidgetView<M>`（含 props、widget_type 等完整信息）和 widget 的 bounds，
/// 应返回该 widget 的 PaintOp 列表。
///
/// 从 `WidgetView.props` 中可提取 `label`、`text`、`checked`、`value` 等属性
/// 构造实际 `WidgetState`，供组件 `paint()` 方法使用。
pub type PaintFn<M> =
    Box<dyn Fn(&rgui_core::view::WidgetView<M>, Rect) -> Vec<PaintOp> + Send + Sync>;

/// 从 WidgetView 树构建 SceneGraph（集成布局引擎）。
///
/// 深度优先遍历 WidgetView 树，通过 `layout_engine` 查询每个 widget
/// 的 Taffy 计算后位置和尺寸作为 bounds，调用 `paint_fn` 获取绘制操作，
/// 组装为图层并构建 SceneGraph。
///
/// # 参数
///
/// - `root`: 根 WidgetView（通常由 `html!` 宏生成）。
/// - `layout_engine`: 布局引擎（需先通过 `compute_view_layout()` 计算布局）。
/// - `paint_fn`: 为每个 widget 生成 PaintOp 的回调。
/// - `version`: 场景图版本号。
/// - `text_renderer`: 可选的文本渲染器。
pub fn build_scene_from_view<M: rgui_core::traits::AppMessage>(
    root: &rgui_core::view::WidgetView<M>,
    layout_engine: &LayoutEngine,
    paint_fn: &PaintFn<M>,
    version: u64,
    text_renderer: Option<&crate::text_renderer::TextRenderer>,
) -> SceneGraph {
    let mut builder = SceneGraphBuilder::new(version);
    let mut z_index: i32 = 0;

    walk_view_tree(
        root,
        layout_engine,
        paint_fn,
        &mut builder,
        &mut z_index,
        text_renderer,
    );

    builder.finish()
}

/// 深度优先遍历 WidgetView 树，收集绘制操作并构建图层。
///
/// 通过 `layout_engine` 查询每个 widget 的 Taffy 计算后位置和尺寸，
/// 而非使用统一的 root_bounds。递归子节点时传递同一 `layout_engine` 引用。
fn walk_view_tree<M: rgui_core::traits::AppMessage>(
    view: &rgui_core::view::WidgetView<M>,
    layout_engine: &LayoutEngine,
    paint_fn: &PaintFn<M>,
    builder: &mut SceneGraphBuilder,
    z_index: &mut i32,
    text_renderer: Option<&crate::text_renderer::TextRenderer>,
) {
    let widget_id = view.id.unwrap_or_else(|| {
        eprintln!(
            "[rgui] walk_view_tree: WidgetView.id 缺失，widget_type=\"{}\"，回退到 WidgetId(0)",
            view.widget_type
        );
        WidgetId::default()
    });

    // 从布局引擎查询该 widget 的计算后 bounds
    let bounds = layout_engine
        .get_layout(widget_id)
        .map(|cached| {
            Rect::new(
                cached.result.position.x,
                cached.result.position.y,
                cached.result.size.width,
                cached.result.size.height,
            )
        })
        .unwrap_or_else(|| {
            eprintln!(
                "[rgui] walk_view_tree: 布局引擎无 WidgetId({widget_id:?}) (widget_type=\"{}\") 的缓存，回退到 Rect::ZERO",
                view.widget_type
            );
            Rect::ZERO
        });

    let ops = paint_fn(view, bounds);

    let mut commands = Vec::with_capacity(ops.len());
    for op in &ops {
        commands.push(paint_op_to_draw_command_inner(op, text_renderer));
    }

    builder.build_layer(widget_id, *z_index, bounds, commands, true);
    *z_index += 1;

    // 递归处理子节点——传递同一 layout_engine 引用
    for child in &view.children {
        walk_view_tree(
            child,
            layout_engine,
            paint_fn,
            builder,
            z_index,
            text_renderer,
        );
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
    text_renderer: Option<&TextRenderer>,
) -> LayoutEngine {
    let mut engine = LayoutEngine::new();
    let root = build_layout_tree(view, &mut engine, text_renderer);
    engine.compute_layout(root, available);
    engine
}

/// DFS 后序遍历 WidgetView 树，为每个节点创建 Taffy layout node。
fn build_layout_tree<M: rgui_core::traits::AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    engine: &mut LayoutEngine,
    text_renderer: Option<&TextRenderer>,
) -> LayoutNode {
    // 后序：先递归子节点
    let child_nodes: Vec<LayoutNode> = view
        .children
        .iter_mut()
        .map(|child| build_layout_tree(child, engine, text_renderer))
        .collect();

    // 从 props 提取 Taffy Style
    let style = extract_taffy_style(view.widget_type, &view.props, text_renderer);

    // 分配唯一 WidgetId
    let widget_id = WidgetId::new();
    view.id = Some(widget_id);

    // 创建 Taffy 节点
    engine.create_node(widget_id, style, &child_nodes)
}

/// 从 `WidgetView.props` 中提取 CSS 布局属性并转换为 Taffy `Style`。
///
/// 首先根据 `widget_type` 注入默认布局样式（如 Center→flex+center、Column→flex+column），
/// 然后用 `props` 中的显式属性覆盖默认值。
///
/// 支持的布局属性：
/// - `display`、`flex-direction`、`justify-content`、`align-items`（字符串）。
/// - `width`、`height`、`gap`、`padding`、`margin`（数值）。
///
/// 未识别的属性或非布局属性（`label`、`text`、`checked` 等）被忽略。
fn extract_taffy_style(
    widget_type: &str,
    props: &std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
    text_renderer: Option<&TextRenderer>,
) -> taffy::Style {
    use rgui_core::view::PropValue;

    // 第一步：根据 widget 类型注入默认布局样式
    let mut style = default_layout_for_type(widget_type);

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

    // 第二步：用 props 中的显式属性覆盖默认值
    if let Some(d) = get_str("display") {
        style.display = rgui_layout::mapping::to_taffy_display(d);
    }
    if let Some(w) = get_f32("width") {
        style.size.width = taffy::Dimension::Length(w);
    }
    if let Some(h) = get_f32("height") {
        style.size.height = taffy::Dimension::Length(h);
    }
    if let Some(fd) = get_str("flex-direction") {
        style.flex_direction = rgui_layout::mapping::to_taffy_flex_direction(fd);
    }
    // WaCheckboxGroup 使用 orientation 而非 flex-direction
    if widget_type == "WaCheckboxGroup" {
        if let Some(orientation) = get_str("orientation") {
            style.flex_direction = match orientation {
                "horizontal" => taffy::FlexDirection::Row,
                _ => taffy::FlexDirection::Column,
            };
        }
    }
    // WaRadioGroup 同理
    if widget_type == "WaRadioGroup" {
        if let Some(orientation) = get_str("orientation") {
            style.flex_direction = match orientation {
                "horizontal" => taffy::FlexDirection::Row,
                _ => taffy::FlexDirection::Column,
            };
        }
    }
    // WaSplitPanel 使用 orientation 控制面板排列方向
    if widget_type == "WaSplitPanel" {
        if let Some(orientation) = get_str("orientation") {
            style.flex_direction = match orientation {
                "vertical" => taffy::FlexDirection::Column,
                _ => taffy::FlexDirection::Row,
            };
        }
    }
    // WaButtonGroup 使用 orientation 控制按钮排列方向
    if widget_type == "WaButtonGroup" {
        if let Some(orientation) = get_str("orientation") {
            style.flex_direction = match orientation {
                "horizontal" => taffy::FlexDirection::Row,
                _ => taffy::FlexDirection::Column,
            };
        }
    }
    // WaCarousel 使用 orientation 控制幻灯片排列方向
    if widget_type == "WaCarousel" {
        if let Some(orientation) = get_str("orientation") {
            style.flex_direction = match orientation {
                "vertical" => taffy::FlexDirection::Column,
                _ => taffy::FlexDirection::Row,
            };
        }
    }
    if let Some(jc) = get_str("justify-content") {
        style.justify_content = Some(rgui_layout::mapping::to_taffy_justify_content(jc));
    }
    if let Some(ai) = get_str("align-items") {
        style.align_items = Some(rgui_layout::mapping::to_taffy_align_items(ai));
    }
    if let Some(g) = get_f32("gap") {
        let length = taffy::LengthPercentage::Length(g);
        style.gap = taffy::geometry::Size {
            width: length,
            height: length,
        };
    }
    if let Some(p) = get_f32("padding") {
        let lp = taffy::LengthPercentage::Length(p);
        style.padding = taffy::geometry::Rect {
            left: lp,
            right: lp,
            top: lp,
            bottom: lp,
        };
    }
    if let Some(m) = get_f32("margin") {
        let lpa = taffy::LengthPercentageAuto::Length(m);
        style.margin = taffy::geometry::Rect {
            left: lpa,
            right: lpa,
            top: lpa,
            bottom: lpa,
        };
    }

    // 第三步：内容驱动尺寸——根据文字内容计算宽度和高度
    if matches!(widget_type, "WaButton" | "WaTab" | "WaBreadcrumbItem" | "Label") {
        let has_explicit_width = props.get("width").is_some();
        let has_explicit_height = props.get("height").is_some();

        if !has_explicit_width || !has_explicit_height {
            if let Some(text) = get_str("label").or(get_str("text")) {
                // Inter Regular: ascent=0.969, descent=-0.227 → em_height=1.196
                let em_height: f32 = 1.196;

                // WaButton 字体大小由 paint() 统一控制: h × 0.44
                // WaTab 同样使用 h × 0.44
                // WaBreadcrumbItem 字体固定 14.0，ratio = 14.0 / min_h(24.0) ≈ 0.583
                let paint_font_ratio: f32 = if widget_type == "WaBreadcrumbItem" {
                    0.583
                } else {
                    0.44
                };

                // ── 高度 ──
                let final_height: f32 = if has_explicit_height {
                    get_f32("height").unwrap_or(40.0)
                } else {
                    let min_h = match style.min_size.height {
                        taffy::Dimension::Length(h) => h,
                        _ => 40.0,
                    };
                    let font_size_from_min = min_h * paint_font_ratio;
                    let text_height_px = font_size_from_min * em_height;
                    let pad_v: f32 = if widget_type == "WaButton" {
                        16.0
                    } else if widget_type == "WaBreadcrumbItem" {
                        8.0
                    } else if widget_type == "WaTab" {
                        17.0
                    } else {
                        4.0
                    };
                    let content_h = text_height_px + pad_v;
                    let h = content_h.max(min_h);
                    style.size.height = taffy::Dimension::Length(h);
                    h
                };

                // ── 宽度 ──
                if !has_explicit_width {
                    let font_size = final_height * paint_font_ratio;
                    let content_width: f32 = if let Some(tr) = text_renderer {
                        let text_px = tr.measure_text(text, font_size);
                        let pad_w: f32 = if widget_type == "WaButton" {
                            32.0
                        } else if widget_type == "WaBreadcrumbItem" {
                            24.0
                        } else if widget_type == "WaTab" {
                            20.0
                        } else {
                            8.0
                        };
                        text_px + pad_w
                    } else {
                        let char_count = text.chars().count().max(1) as f32;
                        let pad_w: f32 = if widget_type == "WaButton" {
                            32.0
                        } else if widget_type == "WaBreadcrumbItem" {
                            24.0
                        } else if widget_type == "WaTab" {
                            20.0
                        } else {
                            8.0
                        };
                        char_count * font_size * 0.6 + pad_w
                    };
                    let min_w = match style.min_size.width {
                        taffy::Dimension::Length(w) => w,
                        _ => 80.0,
                    };
                    style.size.width = taffy::Dimension::Length(content_width.max(min_w));
                }
            }
        }
    }

    style
}

/// 为已知 widget 类型提供默认 Taffy 布局样式。
///
/// 这些默认值确保无显式布局属性的 widget（如 `html!` 宏生成的 `<Center><Button/></Center>`）
/// 仍能正确参与布局计算，而非退化为 0×0 的 Block+Auto 节点。
///
/// # 默认规则
///
/// | Widget 类型 | 默认布局样式 |
/// |-------------|-------------|
/// | `Center` | display: Flex, justify-content: Center, align-items: Center, 100%×100% |
/// | `Column` | display: Flex, flex-direction: Column, 100%×auto |
/// | `Row` | display: Flex, flex-direction: Row, 100%×auto |
/// | `Container`/`Card` | display: Flex, 100%×auto |
/// | `Padding` | display: Flex, 100%×auto, padding: 16px |
/// | `SizedBox` | 无默认（需 props 提供 width/height） |
/// | `Expanded` | flex-grow: 1 |
/// | `ScrollView` | display: Flex, 100%×100% |
/// | `Button` | min-height: 40px, min-width: 80px |
/// | `DataGrid` | display: Flex, 100%×100%（数据密集型组件，默认填充可用空间） |
/// | `ListView` | display: Flex, 100%×100%（可滚动列表，默认填充可用空间） |
/// | `Label` | 无默认（自动适应内容） |
/// | 其他叶子组件 | min-height: 40px |
fn default_layout_for_type(widget_type: &str) -> taffy::Style {
    use taffy::prelude::*;

    let full_size = taffy::geometry::Size {
        width: Dimension::Percent(1.0),
        height: Dimension::Percent(1.0),
    };
    let full_width_auto_height = taffy::geometry::Size {
        width: Dimension::Percent(1.0),
        height: Dimension::Auto,
    };
    let wa_button_min_size = taffy::geometry::Size {
        width: Dimension::Length(80.0),
        height: Dimension::Length(36.0),
    };

    match widget_type {
        // ── 布局容器 ──
        "Center" => Style {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Center),
            size: full_size,
            ..Style::default()
        },
        "Column" => Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: full_width_auto_height,
            ..Style::default()
        },
        "Row" => Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            size: full_width_auto_height,
            ..Style::default()
        },
        "Container" | "Card" | "WaCard" | "WaDetails" | "WaCheckboxGroup" | "WaRadioGroup"
        | "WaTabGroup" | "WaButtonGroup" | "WaSplitPanel" | "WaCarousel" | "Stack" => Style {
            display: Display::Flex,
            size: full_width_auto_height,
            ..Style::default()
        },
        "WaAccordion" => Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: full_width_auto_height,
            ..Style::default()
        },
        "Padding" => Style {
            display: Display::Flex,
            size: full_width_auto_height,
            padding: taffy::geometry::Rect {
                left: LengthPercentage::Length(16.0),
                right: LengthPercentage::Length(16.0),
                top: LengthPercentage::Length(16.0),
                bottom: LengthPercentage::Length(16.0),
            },
            ..Style::default()
        },
        "Expanded" => Style {
            flex_grow: 1.0,
            ..Style::default()
        },
        "ScrollView" => Style {
            display: Display::Flex,
            size: full_size,
            ..Style::default()
        },
        // ── 叶子组件（需要最小尺寸确保在 flex 容器中可见）──
        "WaAvatar" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(48.0),
                height: Dimension::Length(48.0),
            },
            ..Style::default()
        },
        "WaBadge" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(20.0),
                height: Dimension::Length(16.0),
            },
            ..Style::default()
        },
        // WaBreadcrumb：flex 行容器，允许换行，宽度 100% 高度自适应
        "WaBreadcrumb" => Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            size: full_width_auto_height,
            ..Style::default()
        },
        // WaBreadcrumbItem：最小尺寸确保在 flex 容器中可见
        "WaBreadcrumbItem" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(44.0),
                height: Dimension::Length(24.0),
            },
            ..Style::default()
        },
        // WaAccordionItem：标题栏 44px 最小高度，宽度由父容器 Accordion（flex column）驱动
        "WaAccordionItem" => Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: full_width_auto_height,
            min_size: taffy::geometry::Size {
                width: Dimension::Length(200.0),
                height: Dimension::Length(44.0),
            },
            ..Style::default()
        },
        "WaIcon" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(16.0),
                height: Dimension::Length(16.0),
            },
            ..Style::default()
        },
        "WaSpinner" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(24.0),
                height: Dimension::Length(24.0),
            },
            ..Style::default()
        },
        "WaCheckbox" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(24.0),
                height: Dimension::Length(24.0),
            },
            ..Style::default()
        },
        "WaRadio" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(24.0),
                height: Dimension::Length(24.0),
            },
            ..Style::default()
        },
        "WaSwitch" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(48.0),
                height: Dimension::Length(24.0),
            },
            ..Style::default()
        },
        "WaCopyButton" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(32.0),
                height: Dimension::Length(32.0),
            },
            ..Style::default()
        },
        "WaTextarea" => Style {
            size: taffy::geometry::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Auto,
            },
            min_size: taffy::geometry::Size {
                width: Dimension::Length(200.0),
                height: Dimension::Length(100.0),
            },
            ..Style::default()
        },
        "WaSlider" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(200.0),
                height: Dimension::Length(52.0),
            },
            ..Style::default()
        },
        "WaInput" => Style {
            size: taffy::geometry::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Auto,
            },
            min_size: taffy::geometry::Size {
                width: Dimension::Length(120.0),
                height: Dimension::Length(36.0),
            },
            ..Style::default()
        },
        "WaSelect" => Style {
            size: taffy::geometry::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Auto,
            },
            min_size: taffy::geometry::Size {
                width: Dimension::Length(160.0),
                height: Dimension::Length(36.0),
            },
            ..Style::default()
        },
        "WaSkeleton" => Style {
            display: Display::Flex,
            size: full_width_auto_height,
            min_size: taffy::geometry::Size {
                width: Dimension::Length(0.0),
                height: Dimension::Length(16.0),
            },
            ..Style::default()
        },
        "WaProgressBar" => Style {
            display: Display::Flex,
            size: full_width_auto_height,
            min_size: taffy::geometry::Size {
                width: Dimension::Length(0.0),
                height: Dimension::Length(16.0),
            },
            ..Style::default()
        },
        "WaProgressRing" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(48.0),
                height: Dimension::Length(48.0),
            },
            ..Style::default()
        },
        "WaRating" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(120.0),
                height: Dimension::Length(32.0),
            },
            ..Style::default()
        },
        "WaTab" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(64.0),
                height: Dimension::Length(36.0),
            },
            ..Style::default()
        },
        "WaTabPanel" => Style {
            display: Display::Flex,
            size: full_size,
            ..Style::default()
        },
        "WaCarouselItem" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(200.0),
                height: Dimension::Length(100.0),
            },
            ..Style::default()
        },
        "WaCallout" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(200.0),
                height: Dimension::Length(44.0),
            },
            ..Style::default()
        },
        "WaTag" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(20.0),
                height: Dimension::Length(20.0),
            },
            ..Style::default()
        },
        "WaColorPicker" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(36.0),
                height: Dimension::Length(36.0),
            },
            ..Style::default()
        },
        "WaButton" => Style {
            min_size: wa_button_min_size,
            ..Style::default()
        },
        // WaDivider：水平分隔线（宽度 100%，高度 2px 最小）
        "WaDivider" => Style {
            size: taffy::geometry::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Auto,
            },
            min_size: taffy::geometry::Size {
                width: Dimension::Length(0.0),
                height: Dimension::Length(2.0),
            },
            ..Style::default()
        },
        "TextField" | "CheckBox" | "Switch" | "Slider" | "ProgressBar" | "RadioButton"
        | "Image" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(80.0),
                height: Dimension::Length(40.0),
            },
            ..Style::default()
        },
        // ── 数据密集型/可滚动组件——默认填充可用空间 ──
        "DataGrid" | "ListView" => Style {
            size: full_size,
            ..Style::default()
        },
        // Label、Divider、SizedBox 等——无默认，由内容或 props 决定
        _ => {
            eprintln!(
                "[rgui] default_layout_for_type: 未知 widget_type=\"{widget_type}\"，回退到 Style::default() (0×0)"
            );
            Style::default()
        },
    }
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

    /// 辅助函数：创建带 label 属性的 WaButton WidgetView。
    fn make_button(label: &str) -> rgui_core::view::WidgetView<TestMsg> {
        rgui_core::view::WidgetView::new("WaButton")
            .prop("label", rgui_core::view::PropValue::str(label))
    }

    #[test]
    fn compute_view_layout_assigns_unique_ids() {
        let mut view = make_view("Column", vec![make_label("Hello"), make_button("Click")]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

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

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

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

        let _e1 = compute_view_layout(&mut view1, Size::new(200.0, 100.0), None);
        let _e2 = compute_view_layout(&mut view2, Size::new(200.0, 100.0), None);

        // 不同调用分配不同 ID
        assert_ne!(view1.id, view2.id);
        assert_ne!(view1.children[0].id, view2.children[0].id);
    }

    #[test]
    fn compute_view_layout_empty_view() {
        let mut view = make_view("Container", vec![]);

        let engine = compute_view_layout(&mut view, Size::new(800.0, 600.0), None);

        assert!(view.id.is_some());
        let layout = engine.get_layout(view.id.unwrap()).unwrap();
        // 空容器返回有效的布局结果（位置在原点）
        assert!(layout.result.position.x >= 0.0);
        assert!(layout.result.position.y >= 0.0);
    }

    #[test]
    fn button_width_grows_with_label_text() {
        // RED→GREEN: 短文字按钮有最小宽度，长文字按钮精确自适应
        let mut short = make_button("OK");
        let mut long = make_button("Click Me");

        let eng_short = compute_view_layout(&mut short, Size::new(800.0, 600.0), None);
        let eng_long = compute_view_layout(&mut long, Size::new(800.0, 600.0), None);

        let short_width = eng_short
            .get_layout(short.id.unwrap())
            .unwrap()
            .result
            .size
            .width;
        let long_width = eng_long
            .get_layout(long.id.unwrap())
            .unwrap()
            .result
            .size
            .width;

        assert!(
            short_width >= 80.0,
            "短按钮宽度应 ≥ 80px，实际 {short_width}"
        );
        assert!(
            long_width > short_width + 10.0,
            "长按钮 ({long_width}px) 应比短按钮 ({short_width}px) 更宽"
        );

        // 高度：内容驱动，至少 36px（WaButton min_size）
        let short_height = eng_short
            .get_layout(short.id.unwrap())
            .unwrap()
            .result
            .size
            .height;
        assert!(
            short_height >= 36.0,
            "按钮高度应 ≥ 36px，实际 {short_height}"
        );
    }

    // --- build_scene_from_view + layout engine (V03) ---

    /// 简单的 paint_fn：返回空 Vec（仅测试 bounds 传递）。
    fn make_empty_paint_fn() -> PaintFn<TestMsg> {
        Box::new(|_view: &rgui_core::view::WidgetView<TestMsg>, _bounds: Rect| Vec::new())
    }

    /// 记录 bounds 的 paint_fn：用于验证每个 widget 收到的 bounds。
    fn make_recording_paint_fn(
        records: std::sync::Arc<std::sync::Mutex<Vec<(String, Rect)>>>,
    ) -> PaintFn<TestMsg> {
        Box::new(
            move |view: &rgui_core::view::WidgetView<TestMsg>, bounds: Rect| {
                let name = format!(
                    "{}:{}",
                    view.widget_type,
                    view.props
                        .get("text")
                        .or(view.props.get("label"))
                        .map(|v| format!("{v:?}"))
                        .unwrap_or_default()
                );
                records.lock().unwrap().push((name, bounds));
                Vec::new()
            },
        )
    }

    #[test]
    fn build_scene_from_view_with_layout_uses_computed_bounds() {
        // 构造树：Column 包含两个带 height 的 Label（确保 Taffy 能计算不同位置）
        let mut view = make_view(
            "Column",
            vec![
                make_label("Top").prop("height", rgui_core::view::PropValue::from(30.0_f64)),
                make_label("Bottom").prop("height", rgui_core::view::PropValue::from(30.0_f64)),
            ],
        );

        // 计算布局
        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

        // 记录 paint_fn 收到的 bounds
        let records = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let paint_fn = make_recording_paint_fn(std::sync::Arc::clone(&records));

        let _scene = build_scene_from_view(&view, &engine, &paint_fn, 42, None);

        let rec = records.lock().unwrap();
        assert_eq!(rec.len(), 3, "应记录 3 个 widget 的 bounds");

        // 验证每个 widget 收到的 bounds 与 layout engine 一致
        fn verify_bounds(
            view: &rgui_core::view::WidgetView<TestMsg>,
            engine: &LayoutEngine,
            rec: &[(String, Rect)],
        ) {
            let widget_id = view.id.unwrap_or_default();
            if let Some(cached) = engine.get_layout(widget_id) {
                let expected = Rect::new(
                    cached.result.position.x,
                    cached.result.position.y,
                    cached.result.size.width,
                    cached.result.size.height,
                );
                // 用 widget_type + prop 值精确匹配（同类型多 widget 时避免误匹配）
                let type_key = format!(
                    "{}:{}",
                    view.widget_type,
                    view.props
                        .get("text")
                        .or(view.props.get("label"))
                        .map(|v| format!("{v:?}"))
                        .unwrap_or_default()
                );
                let actual = rec.iter().find(|(name, _)| name.starts_with(&type_key));
                if let Some((_name, actual_bounds)) = actual {
                    assert!(
                        (actual_bounds.origin.x - expected.origin.x).abs() < 1.0,
                        "widget {widget_id:?} x mismatch: expected={}, actual={}",
                        expected.origin.x,
                        actual_bounds.origin.x
                    );
                    assert!(
                        (actual_bounds.origin.y - expected.origin.y).abs() < 1.0,
                        "widget {widget_id:?} y mismatch: expected={}, actual={}",
                        expected.origin.y,
                        actual_bounds.origin.y
                    );
                }
            }
            for child in &view.children {
                verify_bounds(child, engine, rec);
            }
        }
        verify_bounds(&view, &engine, &rec);
    }

    #[test]
    fn build_scene_from_view_scene_layers_have_correct_bounds() {
        let mut view = make_view(
            "Column",
            vec![
                make_view("Row", vec![make_button("A"), make_button("B")]),
                make_label("Footer"),
            ],
        );

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);

        // 5 层：Column + Row + 2 Buttons + Footer Label
        assert_eq!(scene.layer_count(), 5);

        // 每个 layer 的 bounds 应与 layout engine 一致
        fn check_layer_bounds(
            view: &rgui_core::view::WidgetView<TestMsg>,
            engine: &LayoutEngine,
            scene: &SceneGraph,
        ) {
            if let Some(id) = view.id {
                let layout = engine.get_layout(id);
                let layer = scene.layers.iter().find(|l| l.widget_id == id);
                match (layout, layer) {
                    (Some(l), Some(ly)) => {
                        assert!(
                            (ly.bounds.origin.x - l.result.position.x).abs() < 1.0,
                            "x mismatch for {id:?}"
                        );
                        assert!(
                            (ly.bounds.origin.y - l.result.position.y).abs() < 1.0,
                            "y mismatch for {id:?}"
                        );
                        assert!(
                            (ly.bounds.size.width - l.result.size.width).abs() < 1.0,
                            "w mismatch for {id:?}"
                        );
                        assert!(
                            (ly.bounds.size.height - l.result.size.height).abs() < 1.0,
                            "h mismatch for {id:?}"
                        );
                    },
                    _ => { /* widget without layout -- skip */ },
                }
            }
            for child in &view.children {
                check_layer_bounds(child, engine, scene);
            }
        }

        check_layer_bounds(&view, &engine, &scene);
    }

    // --- 默认布局样式测试 (V03 bugfix: html! 无显式 props 的 widget 不退化为 0×0) ---

    #[test]
    fn default_layout_center_without_props_is_flex_center() {
        // Center 无显式 props 时应有 Flex+Center 默认样式
        let style = default_layout_for_type("Center");
        assert_eq!(style.display, taffy::Display::Flex);
        assert_eq!(style.justify_content, Some(taffy::JustifyContent::Center));
        assert_eq!(style.align_items, Some(taffy::AlignItems::Center));
        // Center 默认 100%×100% 填充父容器
        assert_eq!(style.size.width, taffy::Dimension::Percent(1.0));
        assert_eq!(style.size.height, taffy::Dimension::Percent(1.0));
    }

    #[test]
    fn default_layout_wa_button_has_minimum_size() {
        let style = default_layout_for_type("WaButton");
        // WaButton 默认有最小尺寸 80×36
        assert_eq!(style.min_size.width, taffy::Dimension::Length(80.0));
        assert_eq!(style.min_size.height, taffy::Dimension::Length(36.0));
    }

    #[test]
    fn default_layout_column_is_flex_column() {
        let style = default_layout_for_type("Column");
        assert_eq!(style.display, taffy::Display::Flex);
        assert_eq!(style.flex_direction, taffy::FlexDirection::Column);
    }

    #[test]
    fn default_layout_row_is_flex_row() {
        let style = default_layout_for_type("Row");
        assert_eq!(style.display, taffy::Display::Flex);
        assert_eq!(style.flex_direction, taffy::FlexDirection::Row);
    }

    #[test]
    fn default_layout_datagrid_fills_available() {
        // DataGrid 默认 100%×100% 填充父容器（数据密集型组件）
        let style = default_layout_for_type("DataGrid");
        assert_eq!(style.size.width, taffy::Dimension::Percent(1.0));
        assert_eq!(style.size.height, taffy::Dimension::Percent(1.0));
    }

    #[test]
    fn default_layout_listview_fills_available() {
        // ListView 默认 100%×100% 填充父容器（可滚动列表）
        let style = default_layout_for_type("ListView");
        assert_eq!(style.size.width, taffy::Dimension::Percent(1.0));
        assert_eq!(style.size.height, taffy::Dimension::Percent(1.0));
    }

    #[test]
    fn default_layout_unknown_type_returns_default() {
        // 未知 widget 类型回退到 Taffy 默认 Style（Flex + Auto）
        let style = default_layout_for_type("UnknownWidget");
        assert_eq!(style.display, taffy::Display::Flex);
        assert_eq!(style.size.width, taffy::Dimension::Auto);
    }

    #[test]
    fn center_with_button_layout_produces_nonzero_bounds() {
        // 回归测试：Center+Button（均无显式 props）不应退化为 0×0
        let mut view = make_view("Center", vec![make_button("OK")]);

        let engine = compute_view_layout(&mut view, Size::new(300.0, 200.0), None);

        // Center 应填充全窗口
        let center_id = view.id.unwrap();
        let center_layout = engine.get_layout(center_id).unwrap();
        assert!(
            center_layout.result.size.width > 10.0,
            "Center width should be >10, got {}",
            center_layout.result.size.width
        );
        assert!(
            center_layout.result.size.height > 10.0,
            "Center height should be >10, got {}",
            center_layout.result.size.height
        );

        // Button 应有非零尺寸（至少接近 80×40 的默认值）
        let button_id = view.children[0].id.unwrap();
        let button_layout = engine.get_layout(button_id).unwrap();
        assert!(
            button_layout.result.size.width > 10.0,
            "Button width should be >10, got {}",
            button_layout.result.size.width
        );
        assert!(
            button_layout.result.size.height > 10.0,
            "Button height should be >10, got {}",
            button_layout.result.size.height
        );

        // Button 应被居中放置在 Center 内
        // Center 300×200, Button ~80×40 → x ≈ (300-80)/2 = 110
        let expected_x = (300.0 - button_layout.result.size.width) / 2.0;
        assert!(
            (button_layout.result.position.x - expected_x).abs() < 5.0,
            "Button x should be centered: expected ~{expected_x}, got {}",
            button_layout.result.position.x
        );
    }

    #[test]
    fn props_override_default_layout() {
        // 验证显式 props 覆盖默认布局样式
        let mut view = make_view("Center", vec![make_button("OK")])
            .prop(
                "display",
                rgui_core::view::PropValue::Str(std::sync::Arc::from("block")),
            )
            .prop("width", rgui_core::view::PropValue::from(200.0_f64))
            .prop("height", rgui_core::view::PropValue::from(100.0_f64));

        let engine = compute_view_layout(&mut view, Size::new(800.0, 600.0), None);

        let center_id = view.id.unwrap();
        let center_layout = engine.get_layout(center_id).unwrap();
        // 显式尺寸覆盖了默认的 100%
        assert!(
            (center_layout.result.size.width - 200.0).abs() < 1.0,
            "width should be 200, got {}",
            center_layout.result.size.width
        );
        assert!(
            (center_layout.result.size.height - 100.0).abs() < 1.0,
            "height should be 100, got {}",
            center_layout.result.size.height
        );
    }
}
