//! Context 类型——WidgetSpec 各方法的上下文参数。
//!
//! 定义源自 D0 §5.5。各 Context 提供 WidgetSpec 方法所需的
//! 只读环境信息或可变操作句柄。

use crate::geometry::{Rect, Size};
use crate::id::WidgetId;
use crate::view::Color;

// ============================================================================
// ViewContext
// ============================================================================

/// `view()` 的上下文：提供只读的环境信息。
///
/// 定义源自 D0 §5.5，完整定义见 D1 组件模型。
#[derive(Clone, Debug)]
pub struct ViewContext {
    /// 窗口逻辑尺寸。
    pub window_size: Size,
}

impl ViewContext {
    #[must_use]
    pub const fn new(window_size: Size) -> Self {
        Self { window_size }
    }
}

// ============================================================================
// UpdateContext
// ============================================================================

/// `update()` 的上下文：提供状态读写能力。
///
/// 完整定义见 D2 状态管理。
#[derive(Debug)]
pub struct UpdateContext {
    /// 当前焦点 widget ID。
    pub focus: Option<WidgetId>,
}

impl UpdateContext {
    #[must_use]
    pub const fn new() -> Self {
        Self { focus: None }
    }
}

impl Default for UpdateContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MeasureContext
// ============================================================================

/// `measure()` 的上下文：提供 DPI 缩放比例。
///
/// 完整定义见 D3 渲染管线（布局阶段）。
#[derive(Clone, Debug)]
pub struct MeasureContext {
    /// 当前 DPI 缩放比例。
    pub scale_factor: f64,
}

impl MeasureContext {
    #[must_use]
    pub fn new(scale_factor: f64) -> Self {
        Self { scale_factor }
    }
}

impl Default for MeasureContext {
    fn default() -> Self {
        Self::new(1.0)
    }
}

// ============================================================================
// PaintOp
// ============================================================================

/// 绘制操作——`paint()` 期间向 PaintContext 发出的绘制指令。
///
/// 定义源自 D3 §5（绘制阶段）。`PaintOp` 是 `paint()` 的输出单位，
/// 由渲染后端转换为具体 DrawCommand。
#[derive(Debug, Clone, PartialEq)]
pub enum PaintOp {
    /// 填充矩形。
    FillRect {
        /// 矩形区域。
        rect: Rect,
        /// 填充颜色。
        color: Color,
        /// 圆角半径（0.0 表示直角）。
        radius: f32,
    },
    /// 绘制文本。
    DrawText {
        /// 文本内容。
        text: String,
        /// 文本边界矩形（左上角对齐基线起始位置）。
        bounds: Rect,
        /// 文本颜色。
        color: Color,
        /// 字体逻辑大小（像素单位，未乘 DPI 缩放因子）。
        font_size: f32,
    },
}

// ============================================================================
// PaintContext
// ============================================================================

/// `paint()` 的上下文：提供裁剪区域，收集绘制操作。
///
/// 组件在 `paint()` 中通过方法（`fill_rect`、`draw_text`）向上下文
/// 提交绘制操作。渲染后端在 `paint()` 返回后消费这些操作，
/// 并转换为 GPU 可执行的 DrawCommand。
///
/// 完整定义见 D3 渲染管线（绘制阶段）。
#[derive(Debug)]
pub struct PaintContext {
    /// 当前裁剪区域。
    pub clip_rect: Rect,
    /// 收集的绘制操作。
    operations: Vec<PaintOp>,
}

impl PaintContext {
    /// 创建新的绘制上下文。
    ///
    /// 预分配 4 个操作的容量——典型 widget（如 Button）
    /// 的 `paint()` 生成 2-5 个操作，避免热路径上多次重新分配。
    #[must_use]
    pub fn new(clip_rect: Rect) -> Self {
        Self {
            clip_rect,
            operations: Vec::with_capacity(4),
        }
    }

    /// 提交填充矩形操作。
    ///
    /// 矩形坐标相对于当前 widget 的本地坐标系（原点为左上角）。
    pub fn fill_rect(&mut self, rect: Rect, color: Color, radius: f32) {
        self.operations.push(PaintOp::FillRect {
            rect,
            color,
            radius,
        });
    }

    /// 提交文本绘制操作。
    ///
    /// `bounds` 定义了文本的布局区域，实际渲染位置由渲染引擎
    /// 根据文本对齐和字体度量计算。
    pub fn draw_text(&mut self, text: &str, bounds: Rect, color: Color, font_size: f32) {
        self.operations.push(PaintOp::DrawText {
            text: text.to_string(),
            bounds,
            color,
            font_size,
        });
    }

    /// 返回已收集的绘制操作（只读）。
    #[must_use]
    pub fn operations(&self) -> &[PaintOp] {
        &self.operations
    }

    /// 消费上下文，返回收集的绘制操作。
    #[must_use]
    pub fn into_operations(self) -> Vec<PaintOp> {
        self.operations
    }

    /// 返回当前已收集的操作数量。
    #[must_use]
    pub fn op_count(&self) -> usize {
        self.operations.len()
    }
}

// ============================================================================
// AccessContext
// ============================================================================

/// `accessibility()` 的上下文：提供可见区域和焦点路径。
///
/// 完整定义见 D6 无障碍系统。
#[derive(Clone, Debug)]
pub struct AccessContext {
    /// 当前可见区域。
    pub visible_bounds: Rect,
    /// 从根节点到当前 widget 的焦点路径。
    pub focus_path: Vec<WidgetId>,
}

impl AccessContext {
    #[must_use]
    pub fn new(visible_bounds: Rect) -> Self {
        Self {
            visible_bounds,
            focus_path: Vec::new(),
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_context_creation() {
        let ctx = ViewContext::new(Size::new(800.0, 600.0));
        assert_eq!(ctx.window_size, Size::new(800.0, 600.0));
    }

    #[test]
    fn update_context_default_focus_is_none() {
        let ctx = UpdateContext::default();
        assert_eq!(ctx.focus, None);
    }

    #[test]
    fn measure_context_default_scale() {
        let ctx = MeasureContext::default();
        assert!((ctx.scale_factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn paint_context_clip_rect() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let ctx = PaintContext::new(rect);
        assert_eq!(ctx.clip_rect, rect);
    }

    #[test]
    fn paint_context_starts_empty() {
        let ctx = PaintContext::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(ctx.op_count(), 0);
        assert!(ctx.operations().is_empty());
    }

    #[test]
    fn paint_context_fill_rect() {
        let mut ctx = PaintContext::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        let rect = Rect::new(10.0, 10.0, 80.0, 40.0);
        ctx.fill_rect(rect, Color::RED, 4.0);
        assert_eq!(ctx.op_count(), 1);
        let ops = ctx.into_operations();
        assert_eq!(
            ops[0],
            PaintOp::FillRect {
                rect,
                color: Color::RED,
                radius: 4.0,
            }
        );
    }

    #[test]
    fn paint_context_draw_text() {
        let mut ctx = PaintContext::new(Rect::new(0.0, 0.0, 200.0, 50.0));
        let bounds = Rect::new(0.0, 0.0, 200.0, 50.0);
        ctx.draw_text("Hello", bounds, Color::BLACK, 14.0);
        assert_eq!(ctx.op_count(), 1);
        let ops = ctx.into_operations();
        assert_eq!(
            ops[0],
            PaintOp::DrawText {
                text: "Hello".to_string(),
                bounds,
                color: Color::BLACK,
                font_size: 14.0,
            }
        );
    }

    #[test]
    fn paint_context_multiple_ops() {
        let mut ctx = PaintContext::new(Rect::new(0.0, 0.0, 300.0, 100.0));
        ctx.fill_rect(Rect::new(0.0, 0.0, 300.0, 100.0), Color::WHITE, 8.0);
        ctx.draw_text(
            "Click me",
            Rect::new(4.0, 4.0, 292.0, 92.0),
            Color::BLACK,
            16.0,
        );
        assert_eq!(ctx.op_count(), 2);
        let ops = ctx.into_operations();
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn paint_op_partial_eq() {
        let op1 = PaintOp::FillRect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color: Color::RED,
            radius: 0.0,
        };
        let op2 = PaintOp::FillRect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color: Color::RED,
            radius: 0.0,
        };
        assert_eq!(op1, op2);
    }

    #[test]
    fn paint_op_clone() {
        let op = PaintOp::DrawText {
            text: "test".into(),
            bounds: Rect::ZERO,
            color: Color::BLUE,
            font_size: 12.0,
        };
        let cloned = op.clone();
        assert_eq!(op, cloned);
    }

    #[test]
    fn access_context_empty_focus_path() {
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let ctx = AccessContext::new(bounds);
        assert_eq!(ctx.visible_bounds, bounds);
        assert!(ctx.focus_path.is_empty());
    }
}
