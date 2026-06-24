//! 全局焦点指示器——自动为当前焦点 widget 绘制轮廓边框。
//!
//! 对应 D6 §5.2（WCAG 2.4.7 焦点可见）：
//! > 框架在场景图生成阶段自动为持有焦点的 widget 追加 2px 主色轮廓边框
//! >（`DrawCommand::StrokePath`），不依赖组件自行实现。
//!
//! # 类型设计说明
//!
//! 本模块中的 `f32` 与 `f64` 转换遵循以下约定：
//! - `f64` 用于矩形边界和布局坐标（与 `rgui_core::geometry` 一致）。
//! - `f32` 用于渲染视觉效果（路径命令坐标、描边宽度、圆角半径）。
//! - `expand_rect()` 是 `f64` 域与 `f32` 域之间的桥接点：
//!   配置项（`outline_offset` 为 `f32`）转为 `f64` 参与边界计算，
//!   随后 `build_rounded_rect_path()` 将结果转为 `f32` 用于路径命令。
//!   `f64` 可精确表示所有 `f32` 值，此转换链无损。

use rgui_core::Color;
use rgui_core::geometry::Rect;
use rgui_core::id::WidgetId;

use crate::primitives::{FillRule, LineCap, LineJoin, Paint, PathCommand, PathData, Stroke};
use crate::scene::{DrawCommand, SceneGraph, SceneGraphBuilder, SceneLayer};

/// 焦点指示器 z 轴顺序（高于所有正常 widget 层）。
const FOCUS_Z_INDEX: i32 = 1_000_000;

/// 焦点指示器配置。
///
/// 控制焦点 outline 的视觉样式（宽度、颜色、圆角半径、偏移量）。
///
/// # 示例
///
/// ```ignore
/// let indicator = FocusIndicator::default();
/// indicator.inject_outline(&mut builder, focus, |id| bounds_map.get(&id).copied());
/// ```
#[derive(Clone, Debug)]
pub struct FocusIndicator {
    /// outline 线条宽度（像素），默认 2.0。
    pub outline_width: f32,

    /// outline 颜色，默认蓝色。
    pub outline_color: Color,

    /// outline 圆角半径（像素），默认 2.0。
    pub corner_radius: f32,

    /// outline 与 widget 边界之间的间距（像素），默认 1.0。
    pub outline_offset: f32,
}

impl Default for FocusIndicator {
    fn default() -> Self {
        Self {
            outline_width: 2.0,
            // 临时蓝色，未来将通过主题系统获取主色
            outline_color: Color::BLUE,
            corner_radius: 2.0,
            outline_offset: 1.0,
        }
    }
}

impl FocusIndicator {
    /// 创建默认配置的焦点指示器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 向场景图中注入焦点 outline（使用 SceneGraphBuilder）。
    ///
    /// # 参数
    ///
    /// * `scene` - 场景图构建器，outline 以新图层形式注入。
    /// * `focused_widget` - 当前焦点 widget 的 ID。为 `None` 时不执行任何操作。
    /// * `get_bounds` - 根据 WidgetId 查询 widget 布局边界的纯查询闭包（不应有副作用）。
    ///
    /// 当 `focused_widget` 为 `Some` 且 `get_bounds` 返回 `Some(bounds)` 时，
    /// 在焦点 widget 边界外追加一个带圆角的 StrokePath 轮廓。
    pub fn inject_outline(
        &self,
        scene: &mut SceneGraphBuilder,
        focused_widget: Option<WidgetId>,
        get_bounds: impl Fn(WidgetId) -> Option<Rect>,
    ) {
        let Some((focus_id, outline_rect, path, stroke, paint)) =
            self.build_outline_layer_data(focused_widget, get_bounds)
        else {
            return;
        };

        scene.build_layer(
            focus_id,
            FOCUS_Z_INDEX,
            outline_rect,
            vec![DrawCommand::StrokePath {
                path,
                stroke,
                paint,
            }],
            true,
        );
    }

    /// 向已构建的 `SceneGraph` 中注入焦点 outline（后处理路径）。
    ///
    /// 用于在场景图已由 `build_scene_from_view` 等函数构建完成后，
    /// 根据焦点状态追加 outline 层。与 `inject_outline` 的区别是
    /// 直接操作 `SceneGraph.layers` 而非通过 `SceneGraphBuilder`。
    ///
    /// # 参数
    ///
    /// * `scene` - 已构建的场景图，outline 层将追加到 `layers` 末尾。
    /// * `focused_widget` - 当前焦点 widget 的 ID。
    /// * `get_bounds` - 根据 WidgetId 查询布局边界的闭包。
    ///
    /// 当焦点 widget 存在且 bounds 可查询时，追加焦点 outline 层。
    pub fn inject_into_scene(
        &self,
        scene: &mut SceneGraph,
        focused_widget: Option<WidgetId>,
        get_bounds: impl Fn(WidgetId) -> Option<Rect>,
    ) {
        let Some((focus_id, outline_rect, path, stroke, paint)) =
            self.build_outline_layer_data(focused_widget, get_bounds)
        else {
            return;
        };

        scene.layers.push(SceneLayer {
            z_index: FOCUS_Z_INDEX,
            bounds: outline_rect,
            commands: vec![DrawCommand::StrokePath {
                path,
                stroke,
                paint,
            }],
            widget_id: focus_id,
            opacity: 1.0,
            transform: None,
        });
    }

    /// 构建 outline 层的绘制数据（内部辅助方法，消除 inject_outline 和 inject_into_scene 的重复代码）。
    ///
    /// 返回 `(focus_id, outline_rect, path, stroke, paint)` 或 `None`（无 focus 或 bounds 缺失）。
    fn build_outline_layer_data(
        &self,
        focused_widget: Option<WidgetId>,
        get_bounds: impl Fn(WidgetId) -> Option<Rect>,
    ) -> Option<(WidgetId, Rect, PathData, Stroke, Paint)> {
        let focus_id = focused_widget?;
        let bounds = get_bounds(focus_id)?;

        let outline_rect = expand_rect(bounds, self.outline_offset as f64);
        let path = build_rounded_rect_path(outline_rect, self.corner_radius);
        let stroke = Stroke {
            width: self.outline_width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            dash_pattern: None,
            dash_offset: 0.0,
        };
        let paint = Paint::Solid(self.outline_color);

        Some((focus_id, outline_rect, path, stroke, paint))
    }

    /// 带标准 focus 偏移的默认 outline rect 快捷创建。
    ///
    /// 等价于 `expand_rect(widget_bounds, self.outline_offset)`：
    /// 返回向外扩展了 `self.outline_offset` 像素（所有方向）的矩形。
    #[inline]
    #[must_use]
    pub fn outline_rect_for(&self, widget_bounds: Rect) -> Rect {
        expand_rect(widget_bounds, self.outline_offset as f64)
    }
}

/// 将矩形向外扩展 `offset` 像素（所有方向）。
#[must_use]
fn expand_rect(r: Rect, offset: f64) -> Rect {
    Rect::new(
        r.origin.x - offset,
        r.origin.y - offset,
        r.size.width + 2.0 * offset,
        r.size.height + 2.0 * offset,
    )
}

/// 构建圆角矩形的路径命令序列。
///
/// 路径方向为顺时针，从左上角开始：
/// 1. 上边（左→右）
/// 2. 右上角（QuadTo）
/// 3. 右边（上→下）
/// 4. 右下角（QuadTo）
/// 5. 下边（右→左）
/// 6. 左下角（QuadTo）
/// 7. 左边（下→上）
/// 8. 左上角（QuadTo）
/// 9. Close
///
/// 当半径 ≤ 0 或矩形尺寸不足以容纳圆角时，返回普通矩形路径。
#[must_use]
fn build_rounded_rect_path(rect: Rect, radius: f32) -> PathData {
    let half_w = (rect.size.width as f32) / 2.0;
    let half_h = (rect.size.height as f32) / 2.0;
    let r = if half_w <= 0.0 || half_h <= 0.0 {
        0.0
    } else {
        radius.min(half_w).min(half_h)
    };

    let x = rect.origin.x as f32;
    let y = rect.origin.y as f32;
    let w = rect.size.width as f32;
    let h = rect.size.height as f32;

    let commands = if r <= 0.0 {
        // 无圆角矩形
        vec![
            PathCommand::MoveTo { x, y },
            PathCommand::LineTo { x: x + w, y },
            PathCommand::LineTo { x: x + w, y: y + h },
            PathCommand::LineTo { x, y: y + h },
            PathCommand::Close,
        ]
    } else {
        // 圆角矩形：顺时针方向
        vec![
            // 上边
            PathCommand::MoveTo { x: x + r, y },
            PathCommand::LineTo { x: x + w - r, y },
            // 右上圆角
            PathCommand::QuadTo {
                cx: x + w,
                cy: y,
                x: x + w,
                y: y + r,
            },
            // 右边
            PathCommand::LineTo {
                x: x + w,
                y: y + h - r,
            },
            // 右下圆角
            PathCommand::QuadTo {
                cx: x + w,
                cy: y + h,
                x: x + w - r,
                y: y + h,
            },
            // 下边
            PathCommand::LineTo { x: x + r, y: y + h },
            // 左下圆角
            PathCommand::QuadTo {
                cx: x,
                cy: y + h,
                x,
                y: y + h - r,
            },
            // 左边
            PathCommand::LineTo { x, y: y + r },
            // 左上圆角
            PathCommand::QuadTo {
                cx: x,
                cy: y,
                x: x + r,
                y,
            },
            PathCommand::Close,
        ]
    };

    PathData {
        commands,
        fill_rule: FillRule::NonZero,
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 辅助测试：从 bounds_map 创建查询闭包。
    fn bounds_lookup<'a>(
        map: &'a HashMap<WidgetId, Rect>,
    ) -> impl Fn(WidgetId) -> Option<Rect> + 'a {
        |id: WidgetId| map.get(&id).copied()
    }

    // ============================================================
    // FocusIndicator 基本功能
    // ============================================================

    #[test]
    fn focus_indicator_defaults() {
        let fi = FocusIndicator::default();
        assert_eq!(fi.outline_width, 2.0);
        assert_eq!(fi.outline_color, Color::BLUE);
        assert_eq!(fi.corner_radius, 2.0);
        assert_eq!(fi.outline_offset, 1.0);
    }

    #[test]
    fn focus_indicator_new_is_default() {
        let fi1 = FocusIndicator::new();
        let fi2 = FocusIndicator::default();
        assert_eq!(fi1.outline_width, fi2.outline_width);
        assert_eq!(fi1.outline_color, fi2.outline_color);
    }

    #[test]
    fn focus_indicator_clone() {
        let fi = FocusIndicator::default();
        let cloned = fi.clone();
        assert_eq!(fi.outline_width, cloned.outline_width);
        assert_eq!(fi.outline_color, cloned.outline_color);
    }

    // ============================================================
    // inject_outline 行为
    // ============================================================

    #[test]
    fn no_focus_no_outline() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let map = HashMap::new();

        fi.inject_outline(&mut builder, None, bounds_lookup(&map));
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 0);
    }

    #[test]
    fn focus_without_bounds_no_outline() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();
        let map = HashMap::new();

        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 0);
    }

    #[test]
    fn focus_with_zero_bounds_produces_outline() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();
        let mut map = HashMap::new();
        // 零矩形边界也会产生 outline（极小轮廓）
        map.insert(id, Rect::ZERO);

        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 1);
    }

    #[test]
    fn focus_with_bounds_injects_stroke_path() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id, Rect::new(50.0, 30.0, 200.0, 100.0));

        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        let sg = builder.finish();

        assert_eq!(sg.layer_count(), 1);
        let layer = &sg.layers[0];
        assert_eq!(layer.widget_id, id);
        assert_eq!(layer.z_index, FOCUS_Z_INDEX);

        assert_eq!(layer.commands.len(), 1);
        match &layer.commands[0] {
            DrawCommand::StrokePath {
                path,
                stroke,
                paint,
            } => {
                assert_eq!(stroke.width, 2.0);
                assert!(path.commands.len() >= 4);
                match paint {
                    Paint::Solid(color) => assert_eq!(*color, Color::BLUE),
                    _ => panic!("预期 Solid paint"),
                }
            },
            other => panic!("预期 StrokePath，得到 {other:?}"),
        }
    }

    #[test]
    fn focus_outline_bounds_expanded_by_offset() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator {
            outline_offset: 3.0,
            ..Default::default()
        };
        let id = WidgetId::new();
        let mut map = HashMap::new();
        let widget_bounds = Rect::new(100.0, 200.0, 50.0, 30.0);
        map.insert(id, widget_bounds);

        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        let sg = builder.finish();

        let expected = Rect::new(97.0, 197.0, 56.0, 36.0); // 每边扩展 3px
        assert_eq!(sg.layers[0].bounds, expected);
    }

    #[test]
    fn multiple_calls_inject_multiple_layers() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id, Rect::new(0.0, 0.0, 100.0, 100.0));

        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        let sg = builder.finish();

        assert_eq!(sg.layer_count(), 2);
    }

    #[test]
    fn custom_config_used_in_stroke() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator {
            outline_width: 4.0,
            outline_color: Color::RED,
            ..Default::default()
        };
        let id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id, Rect::new(0.0, 0.0, 100.0, 100.0));

        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        let sg = builder.finish();

        match &sg.layers[0].commands[0] {
            DrawCommand::StrokePath { stroke, paint, .. } => {
                assert_eq!(stroke.width, 4.0);
                match paint {
                    Paint::Solid(color) => assert_eq!(*color, Color::RED),
                    _ => panic!("预期 Solid paint"),
                }
            },
            _ => panic!("预期 StrokePath"),
        }
    }

    // ============================================================
    // outline_rect_for
    // ============================================================

    #[test]
    fn outline_rect_for_default_offset() {
        let fi = FocusIndicator::default();
        let r = fi.outline_rect_for(Rect::new(10.0, 20.0, 100.0, 50.0));
        assert_eq!(r.origin.x, 9.0); // 10 - 1
        assert_eq!(r.origin.y, 19.0); // 20 - 1
        assert_eq!(r.size.width, 102.0); // 100 + 2
        assert_eq!(r.size.height, 52.0); // 50 + 2
    }

    #[test]
    fn outline_rect_for_custom_offset() {
        let fi = FocusIndicator {
            outline_offset: 5.0,
            ..Default::default()
        };
        let r = fi.outline_rect_for(Rect::new(0.0, 0.0, 200.0, 100.0));
        assert_eq!(r.origin.x, -5.0);
        assert_eq!(r.origin.y, -5.0);
        assert_eq!(r.size.width, 210.0);
        assert_eq!(r.size.height, 110.0);
    }

    // ============================================================
    // 路径生成
    // ============================================================

    #[test]
    fn path_zero_radius_has_four_segments() {
        let path = build_rounded_rect_path(Rect::new(0.0, 0.0, 100.0, 50.0), 0.0);
        // MoveTo + 3x LineTo + Close = 5 commands
        assert_eq!(path.commands.len(), 5);
        assert!(matches!(path.commands[0], PathCommand::MoveTo { .. }));
        assert!(matches!(path.commands[1], PathCommand::LineTo { .. }));
        assert!(matches!(path.commands[4], PathCommand::Close));
    }

    #[test]
    fn path_positive_radius_has_rounded_corners() {
        let path = build_rounded_rect_path(Rect::new(0.0, 0.0, 100.0, 50.0), 5.0);
        // MoveTo + LineTo + QuadTo + LineTo + QuadTo + LineTo + QuadTo + LineTo + QuadTo + Close
        assert_eq!(path.commands.len(), 10);
        // 应有 4 个 QuadTo（每个角一个）
        let quad_count = path
            .commands
            .iter()
            .filter(|c| matches!(c, PathCommand::QuadTo { .. }))
            .count();
        assert_eq!(quad_count, 4);
    }

    #[test]
    fn path_radius_clamped_to_half_width() {
        // 矩形宽度为 10，半径 10 应被夹到 5
        let path = build_rounded_rect_path(Rect::new(0.0, 0.0, 10.0, 100.0), 10.0);
        let quad_count = path
            .commands
            .iter()
            .filter(|c| matches!(c, PathCommand::QuadTo { .. }))
            .count();
        assert_eq!(quad_count, 4);
    }

    #[test]
    fn path_radius_clamped_to_half_height() {
        // 矩形高度为 8，半径 10 应被夹到 4
        let path = build_rounded_rect_path(Rect::new(0.0, 0.0, 100.0, 8.0), 10.0);
        let quad_count = path
            .commands
            .iter()
            .filter(|c| matches!(c, PathCommand::QuadTo { .. }))
            .count();
        assert_eq!(quad_count, 4);
    }

    #[test]
    fn path_empty_rect_produces_valid_path() {
        // 零矩形不应导致崩溃
        let path = build_rounded_rect_path(Rect::ZERO, 2.0);
        assert!(!path.commands.is_empty());
        assert!(matches!(path.commands.last(), Some(PathCommand::Close)));
    }

    #[test]
    fn path_negative_rect_produces_valid_path() {
        let path = build_rounded_rect_path(Rect::new(-10.0, -20.0, 30.0, 40.0), 2.0);
        assert!(!path.commands.is_empty());
        assert!(matches!(path.commands.last(), Some(PathCommand::Close)));
    }

    #[test]
    fn path_very_small_rect_clamps_radius() {
        let path = build_rounded_rect_path(Rect::new(0.0, 0.0, 2.0, 2.0), 100.0);
        // 最大半径应为 1.0（半宽/半高）
        assert!(!path.commands.is_empty());
    }

    // ============================================================
    // expand_rect
    // ============================================================

    #[test]
    fn expand_rect_positive_offset() {
        let r = expand_rect(Rect::new(10.0, 20.0, 100.0, 50.0), 2.0);
        assert_eq!(r.origin.x, 8.0);
        assert_eq!(r.origin.y, 18.0);
        assert_eq!(r.size.width, 104.0);
        assert_eq!(r.size.height, 54.0);
    }

    #[test]
    fn expand_rect_zero_offset() {
        let r = expand_rect(Rect::new(10.0, 20.0, 100.0, 50.0), 0.0);
        assert_eq!(r, Rect::new(10.0, 20.0, 100.0, 50.0));
    }

    #[test]
    fn expand_rect_negative_origin() {
        let r = expand_rect(Rect::new(-50.0, -30.0, 100.0, 60.0), 5.0);
        assert_eq!(r.origin.x, -55.0);
        assert_eq!(r.origin.y, -35.0);
        assert_eq!(r.size.width, 110.0);
        assert_eq!(r.size.height, 70.0);
    }

    #[test]
    fn expand_rect_zero_size() {
        let r = expand_rect(Rect::ZERO, 1.0);
        assert_eq!(r.origin.x, -1.0);
        assert_eq!(r.origin.y, -1.0);
        assert_eq!(r.size.width, 2.0);
        assert_eq!(r.size.height, 2.0);
    }

    // ============================================================
    // 集成测试：与 SceneGraphBuilder 完整配合
    // ============================================================

    #[test]
    fn focus_indicator_z_index_on_top() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let widget_id = WidgetId::new();
        let focus_id = WidgetId::new();
        let mut map = HashMap::new();
        let widget_bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        map.insert(focus_id, widget_bounds);

        // 先添加一个正常 widget 层
        builder.build_layer(
            widget_id,
            10,
            widget_bounds,
            vec![DrawCommand::FillRect {
                rect: widget_bounds,
                color: Color::WHITE,
                radius: 0.0,
            }],
            false,
        );

        // 再注入焦点 outline
        fi.inject_outline(&mut builder, Some(focus_id), bounds_lookup(&map));

        let sg = builder.finish();

        // 焦点层应在最后（最高 z_index）
        assert_eq!(sg.layer_count(), 2);
        assert_eq!(sg.layers[0].z_index, 10);
        assert_eq!(sg.layers[1].z_index, FOCUS_Z_INDEX);
    }

    #[test]
    fn multiple_widgets_focus_only_one_outline() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id1 = WidgetId::new();
        let id2 = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id1, Rect::new(0.0, 0.0, 100.0, 100.0));
        map.insert(id2, Rect::new(200.0, 0.0, 100.0, 100.0));

        // 只有 id1 有焦点
        fi.inject_outline(&mut builder, Some(id1), bounds_lookup(&map));
        let sg = builder.finish();

        assert_eq!(sg.layer_count(), 1);
        assert_eq!(sg.layers[0].widget_id, id1);
    }

    #[test]
    fn focus_change_injects_different_layer() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id_a = WidgetId::new();
        let id_b = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id_a, Rect::new(0.0, 0.0, 100.0, 100.0));
        map.insert(id_b, Rect::new(200.0, 0.0, 100.0, 100.0));

        // 第一帧：焦点在 id_a
        fi.inject_outline(&mut builder, Some(id_a), bounds_lookup(&map));
        let sg = builder.finish();
        assert_eq!(sg.layers[0].widget_id, id_a);

        // 第二帧：焦点切换到 id_b
        let mut builder = SceneGraphBuilder::new(2);
        fi.inject_outline(&mut builder, Some(id_b), bounds_lookup(&map));
        let sg = builder.finish();
        assert_eq!(sg.layers[0].widget_id, id_b);
    }

    // ============================================================
    // 焦点丢失时不应有 outline
    // ============================================================

    #[test]
    fn focus_blur_removes_outline() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id, Rect::new(0.0, 0.0, 100.0, 100.0));

        // 有焦点
        fi.inject_outline(&mut builder, Some(id), bounds_lookup(&map));
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 1);

        // 失去焦点
        let mut builder = SceneGraphBuilder::new(2);
        fi.inject_outline(&mut builder, None, bounds_lookup(&map));
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 0);
    }

    // ============================================================
    // 直接闭包变体测试
    // ============================================================

    #[test]
    fn closure_returns_none_for_unknown_id() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();

        // 闭包对任何 ID 返回 None
        fi.inject_outline(&mut builder, Some(id), |_| None);
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 0);
    }

    #[test]
    fn closure_returns_some_for_any_id() {
        let mut builder = SceneGraphBuilder::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();

        fi.inject_outline(&mut builder, Some(id), |_| {
            Some(Rect::new(10.0, 10.0, 80.0, 80.0))
        });
        let sg = builder.finish();
        assert_eq!(sg.layer_count(), 1);
    }

    // ============================================================
    // inject_into_scene 测试（后处理路径 —— AC06）
    // ============================================================

    #[test]
    fn inject_into_scene_adds_layer_to_scene_graph() {
        let mut scene = SceneGraph::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id, Rect::new(50.0, 30.0, 200.0, 100.0));

        assert_eq!(scene.layer_count(), 0);
        fi.inject_into_scene(&mut scene, Some(id), bounds_lookup(&map));
        assert_eq!(scene.layer_count(), 1);
        assert_eq!(scene.layers[0].widget_id, id);
        assert_eq!(scene.layers[0].z_index, FOCUS_Z_INDEX);
    }

    #[test]
    fn inject_into_scene_no_focus_does_nothing() {
        let mut scene = SceneGraph::new(1);
        let fi = FocusIndicator::default();
        let map = HashMap::new();

        fi.inject_into_scene(&mut scene, None, bounds_lookup(&map));
        assert_eq!(scene.layer_count(), 0);
    }

    #[test]
    fn inject_into_scene_multiple_calls() {
        let mut scene = SceneGraph::new(1);
        let fi = FocusIndicator::default();
        let id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id, Rect::new(0.0, 0.0, 100.0, 100.0));

        fi.inject_into_scene(&mut scene, Some(id), bounds_lookup(&map));
        fi.inject_into_scene(&mut scene, Some(id), bounds_lookup(&map));
        assert_eq!(scene.layer_count(), 2);
    }

    #[test]
    fn inject_into_scene_preserves_existing_layers() {
        let mut scene = SceneGraph::new(1);
        // Add an existing layer
        scene.layers.push(SceneLayer {
            z_index: 10,
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            commands: vec![DrawCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                color: Color::WHITE,
                radius: 0.0,
            }],
            widget_id: WidgetId::new(),
            opacity: 1.0,
            transform: None,
        });

        let fi = FocusIndicator::default();
        let focus_id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(focus_id, Rect::new(50.0, 30.0, 200.0, 100.0));

        fi.inject_into_scene(&mut scene, Some(focus_id), bounds_lookup(&map));

        assert_eq!(scene.layer_count(), 2);
        // Original layer preserved
        assert_eq!(scene.layers[0].z_index, 10);
        // Focus outline added
        assert_eq!(scene.layers[1].z_index, FOCUS_Z_INDEX);
    }

    #[test]
    fn inject_into_scene_with_custom_config() {
        let mut scene = SceneGraph::new(1);
        let fi = FocusIndicator {
            outline_width: 4.0,
            outline_color: Color::RED,
            outline_offset: 3.0,
            corner_radius: 0.0,
        };
        let id = WidgetId::new();
        let mut map = HashMap::new();
        map.insert(id, Rect::new(50.0, 30.0, 200.0, 100.0));

        fi.inject_into_scene(&mut scene, Some(id), bounds_lookup(&map));

        assert_eq!(scene.layer_count(), 1);
        let layer = &scene.layers[0];
        assert_eq!(layer.bounds, Rect::new(47.0, 27.0, 206.0, 106.0));
        match &layer.commands[0] {
            DrawCommand::StrokePath { stroke, paint, .. } => {
                assert_eq!(stroke.width, 4.0);
                match paint {
                    Paint::Solid(color) => assert_eq!(*color, Color::RED),
                    _ => panic!("预期 Solid paint"),
                }
            },
            _ => panic!("预期 StrokePath"),
        }
    }
}
