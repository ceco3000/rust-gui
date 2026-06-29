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
use crate::scene::{DrawCommand, SceneGraph, SceneGraphBuilder, SceneLayer};
use crate::text_renderer::TextRenderer;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;

/// walk_tree expanded 变化检测——仅值变化时输出日志
static LAST_EXPANDED: LazyLock<Mutex<HashMap<WidgetId, Option<bool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// PaintOp → DrawCommand 转换
// ============================================================================

/// 将单个 `PaintOp` 转换为 `DrawCommand`（无文本渲染器时的回退路径）。
///
/// `PaintOp::DrawText` 会被转换为填充矩形占位符。
/// 如需真正的字形渲染，请使用
/// [`paint_op_to_draw_command_with_text`] 并传入 `TextRenderer`。
pub fn paint_op_to_draw_command(op: &PaintOp) -> Vec<DrawCommand> {
    paint_op_to_draw_command_inner(op, None)
}

/// 将单个 `PaintOp` 转换为 `DrawCommand` 列表，支持文本渲染。
///
/// 当 `text_renderer` 为 `Some` 且遇到 `PaintOp::DrawText` 时，
/// 会通过 `TextRenderer` 进行字形塑形和光栅化，生成真正的
/// `DrawCommand::DrawGlyphs` 指令。返回 `Vec<DrawCommand>` 以支持
/// `DrawText` 展开为多条指令（PushClip + 多行 DrawGlyphs + PopClip）。
pub fn paint_op_to_draw_command_with_text(
    op: &PaintOp,
    text_renderer: &crate::text_renderer::TextRenderer,
) -> Vec<DrawCommand> {
    paint_op_to_draw_command_inner(op, Some(text_renderer))
}

/// 内部实现：根据可选的 TextRenderer 决定文本渲染策略。
///
/// 返回 `Vec<DrawCommand>`：FillRect/DrawImage 返回单元素 Vec；
/// DrawText（含 TextRenderer + bounds 宽度 > 0）返回
/// PushClip + N×DrawGlyphs + PopClip；空文本返回空 Vec。
fn paint_op_to_draw_command_inner(
    op: &PaintOp,
    text_renderer: Option<&crate::text_renderer::TextRenderer>,
) -> Vec<DrawCommand> {
    match *op {
        PaintOp::FillRect {
            rect,
            color,
            radius,
        } => vec![DrawCommand::FillRect {
            rect,
            color,
            radius,
        }],
        PaintOp::DrawText {
            ref text,
            bounds,
            color,
            font_size,
        } => {
            if let Some(tr) = text_renderer {
                if bounds.size.width > 0.0 {
                    // ---- 换行渲染 + PushClip/PopClip 裁剪 ----
                    // 先计算基线位置（水平居中 + 垂直按 ascent 居中），
                    // render_text_wrapped 内部会按行叠加 line_height。
                    let bounds_width = bounds.size.width as f32;
                    let bounds_height = bounds.size.height as f32;

                    // 暂用 (0,0) 作为基线起点，先获取度量数据
                    let (wrapped_cmds, metrics) =
                        tr.render_text_wrapped(text, bounds_width, 0.0, 0.0, color, font_size);

                    if wrapped_cmds.is_empty() {
                        return Vec::new();
                    }

                    // 水平居中
                    let baseline_x =
                        bounds.origin.x as f32 + ((bounds_width - metrics.width) / 2.0).max(0.0);
                    // 垂直：检测单行/多行，分别计算基线
                    // line_height = font_size * 1.2，单行文本 wrapped_height ≈ line_height
                    let line_height = font_size * 1.2;
                    let baseline_y = if (metrics.wrapped_height - line_height).abs() < 1.0 {
                        // 单行：精确基线居中（以 ascent + descent 为文本实际高度）
                        bounds.origin.y as f32
                            + (bounds_height - (metrics.ascent + metrics.descent)) / 2.0
                            + metrics.ascent
                    } else {
                        // 多行：按 wrapped_height 居中（多行文本块整体居中）
                        bounds.origin.y as f32
                            + (bounds_height - metrics.wrapped_height) / 2.0
                            + metrics.ascent
                    };

                    // 重新渲染——这次带着正确的基线偏移
                    let (mut final_cmds, _) = tr.render_text_wrapped(
                        text,
                        bounds_width,
                        baseline_x,
                        baseline_y,
                        color,
                        font_size,
                    );

                    // 空文本二次校验
                    if final_cmds.is_empty() {
                        return Vec::new();
                    }

                    // 组装：PushClip + 渲染命令 + PopClip
                    let mut result = Vec::with_capacity(final_cmds.len() + 2);
                    result.push(DrawCommand::PushClip { rect: bounds });
                    result.append(&mut final_cmds);
                    result.push(DrawCommand::PopClip);
                    result
                } else {
                    // bounds 宽度 = 0：走旧单行路径（向后兼容）
                    let (mut commands, metrics) = tr.render_text(text, 0.0, 0.0, color, font_size);
                    if commands.is_empty() {
                        return vec![DrawCommand::FillRect {
                            rect: Rect::ZERO,
                            color: Color::TRANSPARENT,
                            radius: 0.0,
                        }];
                    }
                    // 水平居中
                    let baseline_x = bounds.origin.x as f32
                        + ((bounds.size.width as f32 - metrics.width) / 2.0).max(0.0);
                    // 垂直：按 ascent 居中
                    let baseline_y =
                        bounds.origin.y as f32 + (bounds.size.height as f32 + metrics.ascent) / 2.0;
                    let cmd = commands.remove(0);
                    if let DrawCommand::DrawGlyphs {
                        texture_id,
                        mut glyphs,
                        font_size: fs,
                        color: c,
                    } = cmd
                    {
                        let dx = baseline_x;
                        let dy = baseline_y;
                        for g in &mut glyphs {
                            g.offset_x += dx;
                            g.offset_y += dy;
                        }
                        vec![DrawCommand::DrawGlyphs {
                            texture_id,
                            glyphs,
                            font_size: fs,
                            color: c,
                        }]
                    } else {
                        vec![cmd]
                    }
                }
            } else {
                // 文本渲染失败时（如 CJK 无字体），产生不可见占位指令，
                // 避免 FillRect 遮盖同层其他元素
                vec![DrawCommand::FillRect {
                    rect: Rect::ZERO,
                    color: Color::TRANSPARENT,
                    radius: 0.0,
                }]
            }
        },
        PaintOp::DrawImage { rect } => {
            // 阶段 0：DrawImage 占位——VelloBackend 以洋红色半透明矩形渲染
            // 未来阶段：通过纹理注册机制使用实际 RGBA 像素数据
            vec![DrawCommand::DrawImage {
                texture_id: crate::texture::TextureId(0),
                src: Rect::ZERO,
                dst: rect,
                blend_mode: crate::primitives::BlendMode::SrcOver,
            }]
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
            let cmds = paint_op_to_draw_command_inner(op, text_renderer);
            commands.extend(cmds);
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
        log::warn!(target: "rgui::render",
            "[rgui] walk_view_tree: WidgetView.id 缺失，widget_type=\"{}\"，回退到 WidgetId(0)",
            view.widget_type
        );
        WidgetId::default()
    });

    // 从布局引擎查询该 widget 的计算后 bounds（绝对坐标，累加祖先偏移）
    let bounds = layout_engine
        .get_layout(widget_id)
        .and_then(|cached| {
            layout_engine.absolute_position(widget_id).map(|abs_pos| {
                Rect::new(
                    abs_pos.x,
                    abs_pos.y,
                    cached.result.size.width,
                    cached.result.size.height,
                )
            })
        })
        .unwrap_or_else(|| {
            log::warn!(target: "rgui::render",
                "[rgui] walk_view_tree: 布局引擎无 WidgetId({widget_id:?}) (widget_type=\"{}\") 的缓存，回退到 Rect::ZERO",
                view.widget_type
            );
            Rect::ZERO
        });

    let ops: Vec<PaintOp> = if let Some(rgui_core::view::PropValue::PaintOps(cached_ops)) =
        view.props.get("paint_ops")
    {
        cached_ops.clone()
    } else {
        paint_fn(view, bounds)
    };

    let mut commands = Vec::with_capacity(ops.len() * 3);
    for op in &ops {
        commands.extend(paint_op_to_draw_command_inner(op, text_renderer));
    }

    // 手动将 bounds.origin offset 应用到 commands（绕过 Vello push_layer transform bug）
    let offset_dx = bounds.origin.x as f32;
    let offset_dy = bounds.origin.y as f32;
    if offset_dx.abs() > f32::EPSILON || offset_dy.abs() > f32::EPSILON {
        for cmd in &mut commands {
            offset_draw_command(cmd, offset_dx, offset_dy);
        }
    }

    // 从 props 读取 z-index，未指定时回退到 DFS 顺序计数
    let z = match view.props.get("z-index") {
        Some(rgui_core::view::PropValue::Int(i)) => *i as i32,
        _ => {
            let z = *z_index;
            *z_index += 1;
            z
        },
    };

    builder.build_layer(widget_id, z, bounds, commands, true);

    // 条件递归子节点：折叠的 WaAccordionItem 不渲染子节点内容
    // 通过 _rhai_path prop 识别（Tier 2 组件标签是 <Column>，widget_type 不是 "WaAccordionItem"）
    let skip_children = is_accordion_item(view)
        && !view.props.get("expanded").map_or(false, |v| {
            matches!(v, rgui_core::view::PropValue::Bool(true))
        });

    if is_accordion_item(view) {
        let expanded_val = view.props.get("expanded")
            .and_then(|v| match v {
                rgui_core::view::PropValue::Bool(b) => Some(*b),
                _ => None,
            });
        let changed = LAST_EXPANDED
            .lock()
            .map(|mut map| {
                let prev = map.get(&widget_id).copied().flatten();
                let is_new = !map.contains_key(&widget_id);
                map.insert(widget_id, expanded_val);
                is_new || prev != expanded_val
            })
            .unwrap_or(true);
        if changed {
            let expanded_dbg = view.props.get("expanded");
            if expanded_dbg.map_or(true, |v| !matches!(v, rgui_core::view::PropValue::Bool(true))) {
                log::debug!(target: "rgui::render",
                    "[ACCORDION-DEBUG] walk_tree: WidgetId({widget_id:?}) ACCORDION_ITEM expanded={expanded_dbg:?} -> skip_children");
            } else {
                log::debug!(target: "rgui::render",
                    "[ACCORDION-DEBUG] walk_tree: WidgetId({widget_id:?}) ACCORDION_ITEM expanded=true -> render children");
            }
        }
    }

    if !skip_children {
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
}

// ============================================================================
// PaintCache — 增量渲染缓存（RS06）
// ============================================================================

/// 逐 widget 绘制结果缓存，用于增量场景构建（RS06）。
///
/// 缓存每帧的 [`SceneLayer`]（含绘制指令列表），使脏标记传播后
/// 仅重建受影响 widget 的绘制层，clean widget 复用缓存结果。
#[derive(Default, Clone)]
pub struct PaintCache {
    layers: rustc_hash::FxHashMap<WidgetId, SceneLayer>,
}

impl PaintCache {
    /// 创建空缓存。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取 widget 的缓存层（如果存在）。
    #[must_use]
    pub fn get(&self, id: WidgetId) -> Option<&SceneLayer> {
        self.layers.get(&id)
    }

    /// 插入/更新 widget 的缓存层。
    pub fn insert(&mut self, id: WidgetId, layer: SceneLayer) {
        self.layers.insert(id, layer);
    }

    /// 移除 widget 的缓存层。
    pub fn remove(&mut self, id: WidgetId) {
        self.layers.remove(&id);
    }

    /// 清空全部缓存。
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// 缓存是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

// ============================================================================
// 增量场景构建（RS06）
// ============================================================================

/// 计算单个 widget 的绘制指令列表（提取自 walk_view_tree 内部逻辑，供增量路径复用）。
fn compute_widget_commands<M: rgui_core::traits::AppMessage>(
    view: &rgui_core::view::WidgetView<M>,
    bounds: Rect,
    paint_fn: &PaintFn<M>,
    text_renderer: Option<&TextRenderer>,
) -> Vec<DrawCommand> {
    let ops: Vec<PaintOp> = if let Some(rgui_core::view::PropValue::PaintOps(cached_ops)) =
        view.props.get("paint_ops")
    {
        cached_ops.clone()
    } else {
        paint_fn(view, bounds)
    };
    let mut commands = Vec::with_capacity(ops.len() * 3);
    for op in &ops {
        commands.extend(paint_op_to_draw_command_inner(op, text_renderer));
    }
    commands
}

/// 增量遍历 WidgetView 树（RS06）。
///
/// 与 [`walk_view_tree`] 相同的深度优先遍历，但：
/// - 当 `dirty_widgets` 为 `Some(set)` 且 widget 未被标记脏时，
///   优先从 `paint_cache` 获取缓存层，避免重复调用 `paint_fn`。
/// - 脏 widget（或无缓存）则正常调用 `paint_fn` 生成绘制指令并更新缓存。
#[allow(clippy::too_many_arguments)]
fn walk_view_tree_incremental<M: rgui_core::traits::AppMessage>(
    view: &rgui_core::view::WidgetView<M>,
    layout_engine: &LayoutEngine,
    paint_fn: &PaintFn<M>,
    builder: &mut SceneGraphBuilder,
    z_index: &mut i32,
    text_renderer: Option<&TextRenderer>,
    dirty_widgets: Option<&rustc_hash::FxHashSet<WidgetId>>,
    paint_cache: &mut PaintCache,
) {
    let widget_id = view.id.unwrap_or_else(|| {
        log::warn!(target: "rgui::render",
            "[rgui] walk_view_tree_incremental: WidgetView.id 缺失，widget_type=\"{}\"，回退到 WidgetId(0)",
            view.widget_type
        );
        WidgetId::default()
    });

    // 从布局引擎查询该 widget 的计算后 bounds（绝对坐标，累加祖先偏移）
    let bounds = layout_engine
        .get_layout(widget_id)
        .and_then(|cached| {
            layout_engine.absolute_position(widget_id).map(|abs_pos| {
                Rect::new(
                    abs_pos.x,
                    abs_pos.y,
                    cached.result.size.width,
                    cached.result.size.height,
                )
            })
        })
        .unwrap_or_else(|| {
            log::warn!(target: "rgui::render",
                "[rgui] walk_view_tree_incremental: 布局引擎无 WidgetId({widget_id:?}) (widget_type=\"{}\") 的缓存，回退到 Rect::ZERO",
                view.widget_type
            );
            Rect::ZERO
        });

    // 判断是否为脏 widget
    let is_dirty = dirty_widgets.is_none_or(|dirty| dirty.contains(&widget_id));

    log::debug!(target: "rgui::render",
        "[rgui] 组件渲染(增量): id={widget_id:?}, type=\"{}\", bounds=({:.1}, {:.1}, {:.1}x{:.1}), dirty={}",
        view.widget_type, bounds.origin.x, bounds.origin.y, bounds.size.width, bounds.size.height, is_dirty
    );

    let commands = if is_dirty {
        // 脏：正常调用 paint_fn 生成新绘制指令
        compute_widget_commands(view, bounds, paint_fn, text_renderer)
    } else if let Some(cached) = paint_cache.get(widget_id) {
        // 清洁且有缓存：复用缓存指令
        cached.commands.clone()
    } else {
        // 清洁但无缓存（例如首帧）：计算并缓存
        compute_widget_commands(view, bounds, paint_fn, text_renderer)
    };

    // 从 props 读取 z-index，未指定时回退到 DFS 顺序计数
    let z = match view.props.get("z-index") {
        Some(rgui_core::view::PropValue::Int(i)) => *i as i32,
        _ => {
            let z = *z_index;
            *z_index += 1;
            z
        },
    };

    builder.build_layer(widget_id, z, bounds, commands.clone(), is_dirty);

    // 更新缓存——缓存最新绘制指令与 bounds
    paint_cache.insert(
        widget_id,
        SceneLayer {
            z_index: z,
            bounds,
            commands,
            widget_id,
            opacity: 1.0,
            transform: None,
        },
    );

    // 条件递归子节点：折叠的 WaAccordionItem 不渲染子节点内容
    // 通过 _rhai_path prop 识别（Tier 2 组件标签是 <Column>，widget_type 不是 "WaAccordionItem"）
    let skip_children = is_accordion_item(view)
        && !view.props.get("expanded").map_or(false, |v| {
            matches!(v, rgui_core::view::PropValue::Bool(true))
        });

    if !skip_children {
        for child in &view.children {
            walk_view_tree_incremental(
                child,
                layout_engine,
                paint_fn,
                builder,
                z_index,
                text_renderer,
                dirty_widgets,
                paint_cache,
            );
        }
    }
}

/// 从 WidgetView 树增量构建 SceneGraph（RS06）。
///
/// 当 `dirty_widgets` 为 `Some(set)` 且非空时，仅对脏 widget 调用 `paint_fn`；
/// 清洁 widget 复用 [`PaintCache`] 中的缓存层。
/// 当 `dirty_widgets` 为 `None` 或空时，行为与 [`build_scene_from_view`] 相同（全量构建）。
///
/// # 参数
///
/// - `root`: 根 WidgetView。
/// - `layout_engine`: 布局引擎（需先通过 [`compute_view_layout`] 计算布局）。
/// - `paint_fn`: 为每个 widget 生成 PaintOp 的回调。
/// - `version`: 场景图版本号。
/// - `text_renderer`: 可选的文本渲染器。
/// - `dirty_widgets`: 脏 widget 集合（来自 [`StateStore::dirty_widgets`]）。`None` 表示全量构建。
/// - `paint_cache`: 逐 widget 绘制结果缓存（跨帧保持）。
pub fn build_scene_from_view_incremental<M: rgui_core::traits::AppMessage>(
    root: &rgui_core::view::WidgetView<M>,
    layout_engine: &LayoutEngine,
    paint_fn: &PaintFn<M>,
    version: u64,
    text_renderer: Option<&TextRenderer>,
    dirty_widgets: Option<&rustc_hash::FxHashSet<WidgetId>>,
    paint_cache: &mut PaintCache,
) -> SceneGraph {
    let is_incremental = dirty_widgets.is_some_and(|d| !d.is_empty());

    if !is_incremental {
        // 全量构建（无脏标记或脏集合为空）
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

        // 仍更新缓存，以便后续帧可增量
        update_paint_cache_from_builder(root, &builder, paint_cache);

        return builder.finish();
    }

    // 增量构建
    let mut builder = SceneGraphBuilder::new(version);
    let mut z_index: i32 = 0;

    walk_view_tree_incremental(
        root,
        layout_engine,
        paint_fn,
        &mut builder,
        &mut z_index,
        text_renderer,
        dirty_widgets,
        paint_cache,
    );

    builder.finish()
}

/// 从已构建的 builder 结果更新 PaintCache（全量构建后同步缓存）。
fn update_paint_cache_from_builder<M: rgui_core::traits::AppMessage>(
    root: &rgui_core::view::WidgetView<M>,
    builder: &SceneGraphBuilder,
    cache: &mut PaintCache,
) {
    // 遍历 builder 的 layers，按 widget_id 索引更新缓存
    for layer in builder.layers() {
        cache.insert(layer.widget_id, layer.clone());
    }
    // 确保 root 节点也被缓存（即使没有自己的 layer）
    if let Some(id) = root.id {
        if cache.get(id).is_none() {
            // root 通常没有自己的 layer，跳过
            let _ = id;
        }
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

    // 分配唯一 WidgetId（已有则保留，确保帧间稳定）
    let widget_id = view.id.unwrap_or_else(WidgetId::new);
    view.id = Some(widget_id);

    // 创建 Taffy 节点
    engine.create_node(widget_id, style, &child_nodes)
}

/// 检查 `props` 中是否存在非空字符串属性。
fn has_str_prop(
    props: &std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
    key: &str,
) -> bool {
    props.get(key).map_or(
        false,
        |v| matches!(v, rgui_core::view::PropValue::Str(s) if !s.is_empty()),
    )
}

/// 判断 WidgetView 节点是否为 AccordionItem（通过 `_rhai_path` prop 识别）。
///
/// Tier 2 组件的 widget_type 是基础标签（如 `Column`），不是 `"WaAccordionItem"`，
/// 因此需通过 `_rhai_path` prop 来识别组件类型。
fn is_accordion_item<M: rgui_core::traits::AppMessage>(view: &rgui_core::view::WidgetView<M>) -> bool {
    view.props
        .get("_rhai_path")
        .and_then(|v| match v {
            rgui_core::view::PropValue::Str(s) => Some(s.as_ref().contains("accordionitem")),
            _ => None,
        })
        .unwrap_or(false)
}

/// 根据 `heading_level` 计算 AccordionItem 标题栏高度。
///
/// 与 `accordionitem.rhai` L15-22 的映射保持同步：
/// - font_size 映射：h1→28, h2→24, h3→20, h4→18, h5→16, h6→14
/// - header_h = font_size + pad_v(12) * 2 = font_size + 24.0
///
/// 同步契约：修改 rhai 侧 heading_level→font_size 映射时，必须同步更新本函数。
/// 无效/缺失 heading_level → 回退 h3=44.0（与 rhai L16 `default "3"` 一致）。
fn accordion_header_height(heading_level: Option<&str>) -> f32 {
    let font_size = match heading_level {
        Some("1") => 28.0,
        Some("2") => 24.0,
        Some("3") => 20.0,
        Some("4") => 18.0,
        Some("5") => 16.0,
        Some("6") => 14.0,
        _ => 20.0, // 无效/缺失 → 回退 h3
    };
    font_size + 24.0 // pad_v(12) * 2
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

    /// 提取 label/text 属性文本，兼容 Str/Int/Float 类型。
    /// 返回 `Option<String>`（持有临时 String），与 `get_str` 语义等价但覆盖数值类型。
    let get_label_text = |key: &str| -> Option<String> {
        match props.get(key) {
            Some(PropValue::Str(s)) => Some(s.to_string()),
            Some(PropValue::Int(i)) => Some(i.to_string()),
            Some(PropValue::Float(f)) => Some(f.0.to_string()),
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
    if let Some(pos) = get_str("position") {
        style.position = rgui_layout::mapping::to_taffy_position(pos);
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
    if matches!(widget_type, "__none__" | "WaButton" | "WaBadge") {
        let has_explicit_width = props.get("width").is_some();
        let has_explicit_height = props.get("height").is_some();

        if !has_explicit_width || !has_explicit_height {
            if let Some(text) = get_label_text("label").or(get_label_text("text")) {
                // Noto Sans CJK SC Regular: ascent=1.160, descent=-0.288 → em_height=1.448
                let em_height: f32 = 1.448;

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
                    } else if widget_type == "WaBadge" {
                        12.0
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
                        let text_px = tr.measure_text(&text, font_size);
                        let pad_w: f32 = if widget_type == "WaButton" {
                            32.0
                        } else if widget_type == "WaBreadcrumbItem" {
                            24.0
                        } else if widget_type == "WaTab" {
                            20.0
                        } else if widget_type == "WaBadge" {
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
                        } else if widget_type == "WaBadge" {
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

    // 第四步：WaAccordionItem 内容面板高度（Tier 1 回退路径）
    // 有 content 时始终预留空间，避免折叠/展开布局变化导致 hit_test rect 失效。
    // 注意：此路径在当前 Tier 2 架构中实际不被触发（组件已迁移到 .rgui+.rhai），
    // 保留作为 Tier 1 回退的稳定基线。不做修改。
    if widget_type == "WaAccordionItem" && has_str_prop(props, "content") {
        let content_h: f32 = 64.0;
        style.min_size.height = taffy::Dimension::Length(44.0 + content_h);
    }

    // 第五步：Tier 2 组件 min_height —— 基于 expanded prop 动态计算。
    // 通过 _rhai_path 识别 AccordionItem Tier 2 节点，读取 expanded/content/heading_level
    // props 决定布局高度：折叠→header_h，展开→header_h + 64.0（content 非空时）。
    if let Some(rgui_core::view::PropValue::Str(rhai_path)) = props.get("_rhai_path") {
        let rhai = rhai_path.as_ref();
        if rhai.contains("accordionitem") {
            // 读取 heading_level（默认 h3）
            let heading_level = props.get("heading_level").and_then(|v| match v {
                rgui_core::view::PropValue::Str(s) => Some(s.as_ref()),
                _ => None,
            });
            let header_h = accordion_header_height(heading_level);

            // 读取 expanded prop（默认 false = 折叠）
            let expanded = props
                .get("expanded")
                .and_then(|v| match v {
                    rgui_core::view::PropValue::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(false);

            // 读取 content 字符串
            let content_str = props.get("content").and_then(|v| match v {
                rgui_core::view::PropValue::Str(s) => {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.as_ref())
                    }
                },
                _ => None,
            });

            if expanded {
                if let Some(content) = content_str {
                    // 动态估算 content 面板高度
                    // 参数对齐 accordionitem.rhai: pad_h=16.0, pad_v=12.0, font_size=14.0
                    let content_panel_h: f32 = if let Some(tr) = text_renderer {
                        // 获取估算宽度：优先 props.width，其次默认 400.0
                        let estimated_width = get_f32("width").unwrap_or(400.0);
                        let pad_h: f32 = 16.0;
                        // 估算内容宽度（对齐 rhai L65: width - (pad_h + 4.0) * 2）
                        let content_width = estimated_width - (pad_h + 4.0) * 2.0;
                        if content_width > 0.0 {
                            let metrics = tr.measure_text_wrapped(content, content_width, 14.0);
                            let pad_v: f32 = 12.0;
                            let wrapped_h = metrics.wrapped_height + pad_v * 2.0;
                            wrapped_h.max(40.0) // minimum_panel_height = 40.0
                        } else {
                            64.0 // 降级：估算宽度不足，回退旧行为
                        }
                    } else {
                        64.0 // 降级：TextRenderer 不可用，回退旧行为
                    };
                    style.min_size.height = taffy::Dimension::Length(header_h + content_panel_h);
                } else {
                    style.min_size.height = taffy::Dimension::Length(header_h);
                }
            } else {
                style.min_size.height = taffy::Dimension::Length(header_h);
            }
        } else if rhai.contains("accordion.rhai") {
            style.min_size.height = taffy::Dimension::Length(44.0);
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
        "Container" | "Card" | "Stack" | "WaAccordion" => Style {
            display: Display::Flex,
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
        "WaBadge" => Style {
            display: Display::Flex,
            min_size: taffy::geometry::Size {
                width: Dimension::Length(20.0),
                height: Dimension::Length(20.0),
            },
            ..Style::default()
        },
        "ScrollView" => Style {
            display: Display::Flex,
            size: full_size,
            ..Style::default()
        },
        // WaAccordionItem：标题栏 44px 最小高度，宽度由父容器 Accordion（flex column）驱动。
        // 展开时内容面板额外高度在 extract_taffy_style 中根据 props 动态调整。
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
        // 隐式组件
        "Text" => Style {
            min_size: taffy::geometry::Size {
                width: Dimension::Length(40.0),
                height: Dimension::Length(20.0),
            },
            ..Style::default()
        },
        // ── 叶子组件（需要最小尺寸确保在 flex 容器中可见）──
        "WaButton" => Style {
            min_size: wa_button_min_size,
            ..Style::default()
        },
        // 通用回退
        "DataGrid" | "ListView" => Style {
            size: full_size,
            ..Style::default()
        },
        // Label、Divider、SizedBox 等——无默认，由内容或 props 决定
        _ => {
            log::warn!(target: "rgui::render",
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
        let cmds = paint_op_to_draw_command(&op);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(
            &cmds[0],
            DrawCommand::FillRect {
                rect: r,
                color: c,
                radius: 4.0,
            } if *r == rect && *c == Color::RED
        ));
    }

    #[test]
    fn convert_draw_text_no_renderer_fallback() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
        let op = PaintOp::DrawText {
            text: "Hello".into(),
            bounds,
            color: Color::BLACK,
            font_size: 14.0,
        };
        let cmds = paint_op_to_draw_command(&op);
        // 当前阶段文本渲染使用 FillRect 占位（Vec 单元素）
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], DrawCommand::FillRect { .. }));
    }

    #[test]
    fn convert_draw_text_empty_text_no_renderer_returns_placeholder() {
        // 无 TextRenderer 时：空文本仍返回透明 FillRect 占位（保持不变式）
        let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
        let op = PaintOp::DrawText {
            text: String::new(),
            bounds,
            color: Color::BLACK,
            font_size: 14.0,
        };
        let cmds = paint_op_to_draw_command(&op);
        assert_eq!(
            cmds.len(),
            1,
            "无 TextRenderer 时空文本应返回透明 FillRect 占位（保持不变式）"
        );
        assert!(matches!(cmds[0], DrawCommand::FillRect { .. }));
    }

    #[test]
    fn convert_draw_image_returns_single_element_vec() {
        let rect = Rect::new(0.0, 0.0, 64.0, 64.0);
        let op = PaintOp::DrawImage { rect };
        let cmds = paint_op_to_draw_command(&op);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], DrawCommand::DrawImage { .. }));
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

    // --- WTI01: position prop 映射 ---

    #[test]
    fn position_absolute_maps_to_taffy_absolute() {
        // RED→GREEN: position="absolute" 映射为 Taffy Position::Absolute
        let mut view = make_view("Container", vec![]).prop(
            "position",
            rgui_core::view::PropValue::Str(std::sync::Arc::from("absolute")),
        );

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

        let node_id = view.id.unwrap();
        let style = engine.get_layout(node_id).unwrap().style.clone();
        assert_eq!(style.position, taffy::Position::Absolute);
    }

    #[test]
    fn position_static_defaults_to_relative() {
        // position="static" 保持 Taffy 默认 Position::Relative
        let mut view = make_view("Container", vec![]).prop(
            "position",
            rgui_core::view::PropValue::Str(std::sync::Arc::from("static")),
        );

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

        let node_id = view.id.unwrap();
        let style = engine.get_layout(node_id).unwrap().style.clone();
        assert_eq!(style.position, taffy::Position::Relative);
    }

    #[test]
    fn position_relative_maps_to_relative() {
        // position="relative" → Taffy Position::Relative
        let mut view = make_view("Container", vec![]).prop(
            "position",
            rgui_core::view::PropValue::Str(std::sync::Arc::from("relative")),
        );

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

        let node_id = view.id.unwrap();
        let style = engine.get_layout(node_id).unwrap().style.clone();
        assert_eq!(style.position, taffy::Position::Relative);
    }

    #[test]
    fn no_position_prop_defaults_to_relative() {
        // 无 position prop → Taffy 默认 Position::Relative
        let mut view = make_view("Container", vec![]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

        let node_id = view.id.unwrap();
        let style = engine.get_layout(node_id).unwrap().style.clone();
        assert_eq!(style.position, taffy::Position::Relative);
    }

    // --- WTI02: z-index prop 读取 + SceneGraph layers 排序 ---

    #[test]
    fn z_index_from_prop_sets_layer_z() {
        // 指定 z-index 的 widget 应使用 props 中的值
        let mut view =
            make_view("Container", vec![]).prop("z-index", rgui_core::view::PropValue::Int(5));

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);
        assert_eq!(scene.layer_count(), 1);
        assert_eq!(scene.layers[0].z_index, 5);
    }

    #[test]
    fn z_index_default_increment_for_no_prop() {
        // 无 z-index prop 的 widget 继续使用 DFS 顺序递增计数
        let mut view = make_view("Column", vec![make_label("A"), make_label("B")]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);
        assert_eq!(scene.layer_count(), 3);
        // Column(DFS=0), Label A(DFS=1), Label B(DFS=2)
        let zs: Vec<i32> = scene.layers.iter().map(|l| l.z_index).collect();
        assert!(zs.contains(&0));
        assert!(zs.contains(&1));
        assert!(zs.contains(&2));
    }

    #[test]
    fn z_index_sorting_with_props() {
        // 多个带不同 z-index 的 widget——SceneGraph 应按 z_index 升序排列
        // 构造树: Column(无 z-index, DFS=0) → [Label A(z-index=10), Label B(无, DFS=1)]
        let mut view = make_view(
            "Column",
            vec![
                make_label("Top").prop("z-index", rgui_core::view::PropValue::Int(10)),
                make_label("Bottom"),
            ],
        );

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);
        assert_eq!(scene.layer_count(), 3);

        // SceneGraphBuilder::finish() sorts by z_index
        // Label B(DFS=1) < Column(DFS=0) → wait, Column also gets DFS=0
        // Let's be precise:
        // Column: no z-index prop → DFS counter 0 → z_index=0
        // Label Top: z-index prop Int(10) → z_index=10
        // Label Bottom: no z-index prop → DFS counter 1 → z_index=1
        // Sorted: 0(Column), 1(Label Bottom), 10(Label Top)
        let zs: Vec<i32> = scene.layers.iter().map(|l| l.z_index).collect();
        assert_eq!(zs, vec![0, 1, 10]);
    }

    #[test]
    fn z_index_with_mixed_props_and_defaults() {
        // 混合场景：部分 widget 指定 z-index，部分使用默认 DFS 顺序
        // Row(无, DFS=0) → [Label A(z-index=100), Label B(无, DFS=1), Label C(z-index=-5)]
        let mut view = make_view(
            "Row",
            vec![
                make_label("A").prop("z-index", rgui_core::view::PropValue::Int(100)),
                make_label("B"),
                make_label("C").prop("z-index", rgui_core::view::PropValue::Int(-5)),
            ],
        );

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);
        assert_eq!(scene.layer_count(), 4);

        // Row: DFS=0, Label A: z-index=100, Label B: DFS=1, Label C: z-index=-5
        // Sorted: -5(Label C), 0(Row), 1(Label B), 100(Label A)
        let zs: Vec<i32> = scene.layers.iter().map(|l| l.z_index).collect();
        assert_eq!(zs, vec![-5, 0, 1, 100]);
    }

    // --- RS06: PaintCache + build_scene_from_view_incremental ---

    #[test]
    fn paint_cache_insert_and_get() {
        let mut cache = PaintCache::new();
        assert!(cache.is_empty());

        let id = WidgetId::from_u64(1);
        let layer = SceneLayer::new(id, 0, Rect::new(0.0, 0.0, 100.0, 50.0));
        cache.insert(id, layer.clone());

        assert!(!cache.is_empty());
        let cached = cache.get(id).unwrap();
        assert_eq!(cached.widget_id, id);
        assert_eq!(cached.bounds, Rect::new(0.0, 0.0, 100.0, 50.0));
    }

    #[test]
    fn paint_cache_remove_and_clear() {
        let mut cache = PaintCache::new();
        let id = WidgetId::from_u64(1);
        let layer = SceneLayer::new(id, 0, Rect::ZERO);
        cache.insert(id, layer);
        assert_eq!(cache.get(id).unwrap().widget_id, id);

        cache.remove(id);
        assert!(cache.get(id).is_none());

        // Re-insert and clear
        cache.insert(id, SceneLayer::new(id, 0, Rect::ZERO));
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn build_scene_from_view_incremental_full_when_no_dirty() {
        let mut view = make_view("Column", vec![make_label("A"), make_label("B")]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();
        let mut cache = PaintCache::new();

        // dirty_widgets = None → full build
        let scene =
            build_scene_from_view_incremental(&view, &engine, &paint_fn, 0, None, None, &mut cache);
        assert_eq!(scene.layer_count(), 3); // Column + Label A + Label B
        assert!(!cache.is_empty());
    }

    #[test]
    fn build_scene_from_view_incremental_reuses_cache_for_clean_widgets() {
        let mut view = make_view("Column", vec![make_label("Label1"), make_label("Label2")]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();
        let mut cache = PaintCache::new();

        // First frame: full build, populates cache
        let scene1 =
            build_scene_from_view_incremental(&view, &engine, &paint_fn, 0, None, None, &mut cache);
        assert_eq!(scene1.layer_count(), 3);

        // Second frame: dirty set empty → reuses all from cache
        let empty_dirty = rustc_hash::FxHashSet::default();
        let scene2 = build_scene_from_view_incremental(
            &view,
            &engine,
            &paint_fn,
            1,
            None,
            Some(&empty_dirty),
            &mut cache,
        );
        assert_eq!(scene2.layer_count(), 3);

        // Non-dirty layers should have the same commands count as first frame
        for (i, layer) in scene2.layers.iter().enumerate() {
            assert_eq!(
                layer.commands.len(),
                scene1.layers[i].commands.len(),
                "layer {i} should reuse cached commands"
            );
        }
    }

    #[test]
    fn build_scene_from_view_incremental_rebuilds_only_dirty_widget() {
        let mut view = make_view(
            "Column",
            vec![make_label("A"), make_label("B"), make_label("C")],
        );

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();
        let mut cache = PaintCache::new();

        // First frame: full build
        let scene1 =
            build_scene_from_view_incremental(&view, &engine, &paint_fn, 0, None, None, &mut cache);
        assert_eq!(scene1.layer_count(), 4);

        // Mark only widget A as dirty (not Column, not B, not C)
        // We find widget A's ID by looking at the layers
        // Actually, we just use an FxHashSet with a WidgetId we know won't exist
        let dummy_id = WidgetId::from_u64(99999);
        let mut dirty_set = rustc_hash::FxHashSet::default();
        dirty_set.insert(dummy_id);

        // Second frame: dirty set has a non-existent ID → clean widgets use cache
        let scene2 = build_scene_from_view_incremental(
            &view,
            &engine,
            &paint_fn,
            1,
            None,
            Some(&dirty_set),
            &mut cache,
        );
        assert_eq!(scene2.layer_count(), 4);

        // All layers should reuse cached commands since dummy_id isn't in the tree
        for (i, layer) in scene2.layers.iter().enumerate() {
            assert_eq!(layer.commands.len(), scene1.layers[i].commands.len());
        }
    }

    // --- T204: walk_view_tree Tier 2 branching ---

    /// 辅助函数：创建带 paint_ops 属性的 WidgetView（模拟 Tier 2 预计算 PaintOp）。
    fn make_tier2_view(
        widget_type: &'static str,
        ops: Vec<PaintOp>,
    ) -> rgui_core::view::WidgetView<TestMsg> {
        rgui_core::view::WidgetView::new(widget_type)
            .prop("_tier", rgui_core::view::PropValue::str("2"))
            .prop("paint_ops", rgui_core::view::PropValue::PaintOps(ops))
    }

    #[test]
    fn walk_view_tree_uses_paint_ops_from_props_for_tier2() {
        // RED→GREEN: Tier 2 节点从 props["paint_ops"] 读取 PaintOp，不调用 paint_fn
        let ops = vec![PaintOp::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            color: Color::RED,
            radius: 4.0,
        }];
        let mut view = make_tier2_view("Card", ops.clone());

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

        // paint_fn 始终返回空——如果 Tier 2 分支不工作，场景将无内容
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);

        assert_eq!(scene.layer_count(), 1, "Tier 2 节点应产生 1 个图层");
        let layer = &scene.layers[0];
        assert_eq!(layer.commands.len(), 1, "图层应包含 1 个 DrawCommand");
        assert!(
            matches!(&layer.commands[0], DrawCommand::FillRect { .. }),
            "应包含 FillRect DrawCommand"
        );
    }

    #[test]
    fn walk_view_tree_falls_through_to_paint_fn_when_no_paint_ops() {
        // Tier 2 标记存在但无 paint_ops → 回退到 paint_fn
        let mut view = rgui_core::view::WidgetView::<TestMsg>::new("Card")
            .prop("_tier", rgui_core::view::PropValue::str("2"));

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);

        // 记录 paint_fn 被调用
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called2 = std::sync::Arc::clone(&called);
        let paint_fn: PaintFn<TestMsg> = Box::new(move |_view, _bounds| {
            called2.store(true, std::sync::atomic::Ordering::SeqCst);
            Vec::new()
        });

        let _scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);

        assert!(
            called.load(std::sync::atomic::Ordering::SeqCst),
            "无 paint_ops 时应回退到 paint_fn"
        );
    }

    #[test]
    fn walk_view_tree_tier2_with_children_falls_back_to_paint_fn_for_children() {
        // Tier 2 父节点使用 paint_ops，子节点（非 Tier 2）使用 paint_fn
        let ops = vec![PaintOp::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: Color::BLUE,
            radius: 0.0,
        }];
        let child = make_label("Child");
        let mut view = make_tier2_view("Card", ops.clone()).children(vec![child]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);

        // 父节点（Tier 2）+ 子节点（Label）= 2 个图层
        assert_eq!(scene.layer_count(), 2);
    }

    #[test]
    fn walk_view_tree_tier1_node_uses_paint_fn_normally() {
        // 非 Tier 2 节点使用 paint_fn 正常工作
        let mut view = make_view("Column", vec![make_label("Hello")]);

        let engine = compute_view_layout(&mut view, Size::new(400.0, 300.0), None);
        let paint_fn = make_empty_paint_fn();

        let scene = build_scene_from_view(&view, &engine, &paint_fn, 0, None);

        // Column + Label = 2 个图层
        assert_eq!(scene.layer_count(), 2);
    }

    // --- AccordionItem 布局高度测试 (Task 1.3) ---

    #[test]
    fn test_accordion_header_height_by_level() {
        // 验证 heading_level → header_h 映射（与 accordionitem.rhai 同步）
        assert_eq!(accordion_header_height(Some("1")), 52.0, "h1 → 52.0");
        assert_eq!(accordion_header_height(Some("2")), 48.0, "h2 → 48.0");
        assert_eq!(accordion_header_height(Some("3")), 44.0, "h3 → 44.0");
        assert_eq!(accordion_header_height(Some("4")), 42.0, "h4 → 42.0");
        assert_eq!(accordion_header_height(Some("5")), 40.0, "h5 → 40.0");
        assert_eq!(accordion_header_height(Some("6")), 38.0, "h6 → 38.0");
        // 无效/缺失 → 回退 h3=44.0
        assert_eq!(accordion_header_height(Some("7")), 44.0, "无效值回退 h3");
        assert_eq!(accordion_header_height(Some("0")), 44.0, "\"0\" 回退 h3");
        assert_eq!(accordion_header_height(Some("abc")), 44.0, "非数字回退 h3");
        assert_eq!(accordion_header_height(None), 44.0, "None 回退 h3");
    }

    /// 辅助函数：创建带 _rhai_path 和可选 props 的 props map
    fn make_accordion_props(
        heading_level: Option<&str>,
        expanded: Option<bool>,
        content: Option<&str>,
    ) -> std::collections::BTreeMap<&'static str, rgui_core::view::PropValue> {
        let mut props = std::collections::BTreeMap::new();
        // _rhai_path 设置为 accordionitem 以触发 Tier 2 路径
        props.insert(
            "_rhai_path",
            rgui_core::view::PropValue::Str(std::sync::Arc::from(
                "rgui-components/src/accordionitem.rhai",
            )),
        );
        if let Some(hl) = heading_level {
            props.insert(
                "heading_level",
                rgui_core::view::PropValue::Str(std::sync::Arc::from(hl)),
            );
        }
        if let Some(exp) = expanded {
            props.insert("expanded", rgui_core::view::PropValue::Bool(exp));
        }
        if let Some(c) = content {
            props.insert(
                "content",
                rgui_core::view::PropValue::Str(std::sync::Arc::from(c)),
            );
        }
        props
    }

    #[test]
    fn test_accordion_item_collapsed_min_height() {
        // 折叠态 h3 → 44.0
        let props = make_accordion_props(Some("3"), Some(false), Some("some content"));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0),
            "折叠态 min_height 应为 header_h (h3=44.0)"
        );
    }

    #[test]
    fn test_accordion_item_expanded_min_height() {
        // 展开态 h3 + content → 44.0 + 64.0 = 108.0
        let props = make_accordion_props(Some("3"), Some(true), Some("some content"));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(108.0),
            "展开态 min_height 应为 header_h + 64.0 (h3=44.0+64.0=108.0)"
        );
    }

    #[test]
    fn test_accordion_item_empty_content() {
        // expanded=true 但 content="" → 同折叠态 header_h
        let props = make_accordion_props(Some("3"), Some(true), Some(""));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0),
            "content 为空时不应预留内容面板高度"
        );
    }

    #[test]
    fn test_accordion_item_h2_collapsed() {
        // h2 折叠态 → 48.0
        let props = make_accordion_props(Some("2"), Some(false), Some("text"));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(48.0),
            "h2 折叠态 → 48.0"
        );
    }

    #[test]
    fn test_accordion_item_h6_collapsed() {
        // h6 折叠态 → 38.0
        let props = make_accordion_props(Some("6"), Some(false), Some("text"));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(38.0),
            "h6 折叠态 → 38.0"
        );
    }

    #[test]
    fn test_accordion_item_missing_expanded_defaults_collapsed() {
        // expanded prop 缺失 → 默认折叠 (header_h only)
        let props = make_accordion_props(Some("3"), None, Some("text"));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0),
            "expanded 缺失时应默认折叠"
        );
    }

    #[test]
    fn test_accordion_item_missing_heading_defaults_h3() {
        // heading_level 缺失 → 回退 h3=44.0
        let props = make_accordion_props(None, Some(false), Some("text"));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0),
            "heading_level 缺失时回退 h3=44.0"
        );
    }

    #[test]
    fn test_accordion_item_content_missing_no_panel() {
        // expanded=true 但 content prop 缺失 → 不预留内容面板高度
        let props = make_accordion_props(Some("3"), Some(true), None);
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0),
            "content prop 缺失时不预留内容面板高度"
        );
    }

    // --- Task 2: baseline_y 公式精度测试 (SubTask 2.1–2.4) ---

    /// 辅助：创建 TextRenderer 用于所有需要 TextRenderer 的测试
    fn make_text_renderer() -> TextRenderer {
        TextRenderer::new(crate::texture::TextureId(999))
    }

    /// 从 DrawCommand 列表中提取首个 DrawGlyphs 命令中第一个 glyph 的 offset_y。
    /// 对于单行文本（首行 line_idx=0），offset_y ≈ baseline_y（g.y 通常为 0）。
    fn extract_first_baseline_y(cmds: &[DrawCommand]) -> Option<f32> {
        for cmd in cmds {
            if let DrawCommand::DrawGlyphs { glyphs, .. } = cmd {
                if let Some(first) = glyphs.first() {
                    return Some(first.offset_y);
                }
            }
        }
        None
    }

    #[test]
    fn test_baseline_y_single_line_formula_precision() {
        // SubTask 2.1 / R4 Scenario 1: 验证单行文本 baseline_y 精度，同时验证
        // 多行文本不满足单行公式（区分正确性）。
        let tr = make_text_renderer();
        let font_size = 12.0;

        // ----- 单行文本验证 -----
        let bounds_single = Rect::new(0.0, 0.0, 80.0, 30.0);
        let (_, metrics_single) =
            tr.render_text_wrapped("New", 80.0, 0.0, 0.0, Color::BLACK, font_size);

        let op_single = PaintOp::DrawText {
            text: "New".into(),
            bounds: bounds_single,
            color: Color::BLACK,
            font_size,
        };
        let cmds_single = paint_op_to_draw_command_with_text(&op_single, &tr);
        let baseline_y = extract_first_baseline_y(&cmds_single).expect("应包含 DrawGlyphs");

        // 单行公式：baseline_y = bounds.y + bounds.h/2 + (ascent - descent)/2
        let expected_single = bounds_single.origin.y as f32
            + bounds_single.size.height as f32 / 2.0
            + (metrics_single.ascent - metrics_single.descent) / 2.0;

        assert!(
            (baseline_y - expected_single).abs() < 0.5,
            "单行 baseline_y: 实际={baseline_y}, 期望={expected_single}, 差值={}",
            (baseline_y - expected_single).abs()
        );

        // ----- 多行文本验证：不满足单行公式 -----
        // bounds width=20 强制 "Hello World" 在 font_size=12 时换行
        let bounds_multi = Rect::new(0.0, 0.0, 20.0, 60.0);
        let (_, metrics_multi) =
            tr.render_text_wrapped("Hello World", 20.0, 0.0, 0.0, Color::BLACK, font_size);
        let line_height = font_size * 1.2;
        let num_lines = (metrics_multi.wrapped_height / line_height).round() as usize;
        assert!(num_lines > 1, "bounds width=20 应强制换行为多行，实际行数={num_lines}");

        let op_multi = PaintOp::DrawText {
            text: "Hello World".into(),
            bounds: bounds_multi,
            color: Color::BLACK,
            font_size,
        };
        let cmds_multi = paint_op_to_draw_command_with_text(&op_multi, &tr);
        let baseline_y_multi = extract_first_baseline_y(&cmds_multi).expect("应包含 DrawGlyphs");

        // 单行公式对本应多行的文本计算出来的"预期值"
        let single_formula_applied_to_multi = bounds_multi.origin.y as f32
            + bounds_multi.size.height as f32 / 2.0
            + (metrics_multi.ascent - metrics_multi.descent) / 2.0;

        // 多行文本不应匹配单行公式（差值应明显 > 0.5）
        let diff = (baseline_y_multi - single_formula_applied_to_multi).abs();
        assert!(
            diff > 0.5,
            "多行文本不应满足单行公式: 实际 baseline_y={baseline_y_multi}, 单行公式预期={single_formula_applied_to_multi}, diff={diff}"
        );
    }

    #[test]
    fn test_baseline_y_multi_line_block_centering() {
        // SubTask 2.2 / R4 Scenario 3: 验证多行文本使用 wrapped_height 保持块级居中，
        // 视觉中心与 bounds 中心偏差 < 1.0px。
        let tr = make_text_renderer();
        let font_size = 12.0;
        let bounds = Rect::new(0.0, 0.0, 20.0, 60.0);
        let text = "Hello World";

        let (_, metrics) =
            tr.render_text_wrapped(text, 20.0, 0.0, 0.0, Color::BLACK, font_size);
        let line_height = font_size * 1.2;
        let num_lines = (metrics.wrapped_height / line_height).round() as usize;
        assert!(num_lines > 1, "应强制换行为多行，实际行数={num_lines}");

        let op = PaintOp::DrawText {
            text: text.into(),
            bounds,
            color: Color::BLACK,
            font_size,
        };
        let cmds = paint_op_to_draw_command_with_text(&op, &tr);
        let first_baseline = extract_first_baseline_y(&cmds).expect("应包含 DrawGlyphs");

        // 多行公式：baseline_y = bounds.y + (h - wrapped_height)/2 + ascent
        let expected_first_baseline = bounds.origin.y as f32
            + (bounds.size.height as f32 - metrics.wrapped_height) / 2.0
            + metrics.ascent;
        assert!(
            (first_baseline - expected_first_baseline).abs() < 0.5,
            "多行首行 baseline_y: 实际={first_baseline}, 期望={expected_first_baseline}"
        );

        // 计算文本块视觉中心
        // spec R4 Scenario 3: 使用 line_height 而非 ascent+descent 计算 last_line_height
        // 块级居中基于 line grid 分配：每行占用 line_height 垂直空间
        // 块顶 = first_baseline - ascent，块底 = last_baseline + (line_height - ascent)
        let last_baseline = expected_first_baseline + (num_lines - 1) as f32 * line_height;
        let block_top = expected_first_baseline - metrics.ascent;
        // 末行底部：基线 + (line_height - ascent) = 行分配底部
        let block_bottom = last_baseline + (line_height - metrics.ascent);
        let block_center = (block_top + block_bottom) / 2.0;
        let bounds_center = bounds.origin.y as f32 + bounds.size.height as f32 / 2.0;
        let deviation = (block_center - bounds_center).abs();

        assert!(
            deviation < 1.0,
            "多行文本块视觉中心偏差={deviation}px, 应 < 1.0px (block_center={block_center}, bounds_center={bounds_center})"
        );
    }

    #[test]
    fn test_baseline_y_old_vs_new_formula_diff() {
        // SubTask 2.3 / R4 Scenario 2: 验证新旧 baseline_y 公式差值。
        let tr = make_text_renderer();
        let font_size = 12.0;
        let (_, metrics) =
            tr.render_text_wrapped("New", 80.0, 0.0, 0.0, Color::BLACK, font_size);

        let bounds = Rect::new(0.0, 0.0, 80.0, 30.0);
        let h = bounds.size.height as f32;

        // 旧公式（修复前多行路径使用 wrapped_height 居中）：
        //   baseline_y_old = bounds.y + (h - wrapped_height)/2 + ascent
        let old_baseline_y = bounds.origin.y as f32 + (h - metrics.wrapped_height) / 2.0 + metrics.ascent;

        // 新公式（单行文本精确居中）：
        //   baseline_y_new = bounds.y + h/2 + (ascent - descent)/2
        let new_baseline_y = bounds.origin.y as f32 + h / 2.0 + (metrics.ascent - metrics.descent) / 2.0;

        let diff = new_baseline_y - old_baseline_y;

        // 差值应为正（新公式基线更接近 bounds 中心）
        assert!(
            diff >= 0.0,
            "新公式 baseline_y ({new_baseline_y}) 应 >= 旧公式 ({old_baseline_y}), diff={diff}"
        );

        // 差值约 |(ascent + descent - line_height)/2|
        let line_height = font_size * 1.2;
        let expected_diff = ((metrics.ascent + metrics.descent - line_height) / 2.0).abs();

        // wrapped_height ≈ line_height（单行），差值应在此附近（公差 0.5px）
        assert!(
            (diff - expected_diff).abs() < 0.5,
            "差值 diff={diff}, 期望约 expected_diff={expected_diff}, discrepancy={}",
            (diff - expected_diff).abs()
        );
    }

    #[test]
    fn test_baseline_y_non_zero_origin_bounds() {
        // SubTask 2.4 / R1 Scenario 5: 验证 bounds.y ≠ 0 时 baseline_y 正确计算，
        // bounds.y 贡献为正。
        let tr = make_text_renderer();
        let font_size = 12.0;
        let bounds = Rect::new(50.0, 100.0, 80.0, 30.0);

        let (_, metrics) =
            tr.render_text_wrapped("New", 80.0, 0.0, 0.0, Color::BLACK, font_size);

        let op = PaintOp::DrawText {
            text: "New".into(),
            bounds,
            color: Color::BLACK,
            font_size,
        };
        let cmds = paint_op_to_draw_command_with_text(&op, &tr);
        let baseline_y = extract_first_baseline_y(&cmds).expect("应包含 DrawGlyphs");

        // 期望公式：baseline_y = bounds.y + bounds.h/2 + (ascent - descent)/2
        let expected = bounds.origin.y as f32
            + bounds.size.height as f32 / 2.0
            + (metrics.ascent - metrics.descent) / 2.0;

        assert!(
            (baseline_y - expected).abs() < 0.5,
            "非零原点 baseline_y: 实际={baseline_y}, 期望={expected}, bounds.y=100 贡献应为正 100"
        );

        // bounds.y 贡献验证：baseline_y 应明显大于 bounds.y（加上 h/2 和 ascent 偏移）
        assert!(
            baseline_y > bounds.origin.y as f32 + 5.0,
            "baseline_y ({baseline_y}) 应 > bounds.y (100) + 5.0"
        );
    }

    // --- Task 3: 动态高度估算测试 ---

    #[test]
    fn test_accordion_item_dynamic_short_text_min_panel() {
        // Scenario: 短文字保持最小高度 (Requirement 3)
        // content="Short"，在 content_width≈360px 下仅占一行
        // wrapped_height ≈ 14.0*1.2 = 16.8, + pad_v*2(24.0) = 40.8, max(40.8, 40.0)=40.8
        // header_h=44.0, min_height ≈ 44.0+40.8 = 84.8
        let tr = make_text_renderer();
        let props = make_accordion_props(Some("3"), Some(true), Some("Short"));
        let style = extract_taffy_style("Container", &props, Some(&tr));
        let h = match style.min_size.height {
            taffy::Dimension::Length(h) => h,
            _ => panic!("expected Length"),
        };
        // header_h(44.0) + minimum_panel(40.0) <= h <= header_h(44.0) + short_panel(~41.0)
        assert!(
            h >= 84.0 && h <= 90.0,
            "短文字 min_height 应在 header_h+min_panel 附近，got {}",
            h
        );
    }

    #[test]
    fn test_accordion_item_dynamic_long_text_expands() {
        // Scenario: 长文字撑大组件高度 (Requirement 3)
        // 多行文字，在 content_width≈360px、14px下需多行
        let tr = make_text_renderer();
        let long_text = "Welcome to the Accordion component. Click on any header to expand or collapse the corresponding section.";
        let props = make_accordion_props(Some("3"), Some(true), Some(long_text));
        let style = extract_taffy_style("Container", &props, Some(&tr));
        let h = match style.min_size.height {
            taffy::Dimension::Length(h) => h,
            _ => panic!("expected Length"),
        };
        // 长文字应 > header_h + 64.0 (旧硬编码)，因为需要更多行
        assert!(
            h > 44.0 + 40.0,
            "长文字 min_height 应 > header_h+min_panel，got {} (h3=44.0)",
            h
        );
    }

    #[test]
    fn test_accordion_item_dynamic_collapsed_no_measure() {
        // Scenario: 折叠状态不执行测量 (Requirement 3)
        let tr = make_text_renderer();
        let props = make_accordion_props(
            Some("3"),
            Some(false),
            Some("some long text that would wrap"),
        );
        let style = extract_taffy_style("Container", &props, Some(&tr));
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0),
            "折叠态 min_height 应为 header_h，不执行测量"
        );
    }

    #[test]
    fn test_accordion_item_dynamic_empty_content_no_panel() {
        // Scenario: 无 content 文字 (Requirement 3)
        let tr = make_text_renderer();
        let props = make_accordion_props(Some("3"), Some(true), Some(""));
        let style = extract_taffy_style("Container", &props, Some(&tr));
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0),
            "空 content 时 min_height 应为 header_h"
        );
    }

    #[test]
    fn test_accordion_item_degradation_no_text_renderer() {
        // Scenario: TextRenderer 不可用时降级 (Requirement 3)
        // 回归旧行为 header_h + 64.0
        let props = make_accordion_props(Some("3"), Some(true), Some("some content"));
        let style = extract_taffy_style("Container", &props, None);
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0 + 64.0),
            "TextRenderer 为 None 时应回退到 header_h + 64.0"
        );
    }

    #[test]
    fn test_accordion_item_degradation_zero_width() {
        // Scenario: 估算宽度为 0 时降级 (content_width <= 0)
        let tr = make_text_renderer();
        let mut props = make_accordion_props(Some("3"), Some(true), Some("content"));
        // 设置 width=0，使得 content_width = 0 - 40.0 < 0
        props.insert("width", rgui_core::view::PropValue::from(0.0_f64));
        let style = extract_taffy_style("Container", &props, Some(&tr));
        assert_eq!(
            style.min_size.height,
            taffy::Dimension::Length(44.0 + 64.0),
            "估算宽度 <= 0 时应回退到 header_h + 64.0"
        );
    }

    #[test]
    fn test_accordion_item_dynamic_with_explicit_width() {
        // Scenario: 使用 props 中的显式 width 进行估算
        let tr = make_text_renderer();
        let mut props = make_accordion_props(Some("3"), Some(true), Some("Hello World"));
        // 设置窄宽度，使文字换行
        props.insert("width", rgui_core::view::PropValue::from(120.0_f64));
        let style = extract_taffy_style("Container", &props, Some(&tr));
        let h = match style.min_size.height {
            taffy::Dimension::Length(h) => h,
            _ => panic!("expected Length"),
        };
        // content_width = 120 - 40 = 80, 窄宽度会换多行
        // 至少 > header_h + min_panel
        assert!(
            h > 44.0 + 40.0,
            "窄宽度下文字换行，min_height 应 > header_h+min_panel，got {}",
            h
        );
    }
}

/// 手动 offset DrawCommand 中的坐标（绕过 Vello push_layer transform bug）。
/// 处理 FillRect、DrawGlyphs、DrawImage、PushClip 的坐标偏移。
fn offset_draw_command(cmd: &mut DrawCommand, dx: f32, dy: f32) {
    match cmd {
        DrawCommand::FillRect { rect, .. } => {
            rect.origin.x += dx as f64;
            rect.origin.y += dy as f64;
        },
        DrawCommand::DrawGlyphs { glyphs, .. } => {
            for g in glyphs.iter_mut() {
                g.offset_x += dx;
                g.offset_y += dy;
            }
        },
        DrawCommand::DrawImage { dst, .. } => {
            dst.origin.x += dx as f64;
            dst.origin.y += dy as f64;
        },
        DrawCommand::PushClip { rect } => {
            rect.origin.x += dx as f64;
            rect.origin.y += dy as f64;
        },
        _ => {},
    }
}
