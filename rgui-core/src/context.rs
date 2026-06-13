//! Context 类型——WidgetSpec 各方法的上下文参数。
//!
//! 定义源自 D0 §5.5。各 Context 提供 WidgetSpec 方法所需的
//! 只读环境信息或可变操作句柄。

use crate::geometry::{Rect, Size};
use crate::id::WidgetId;

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
// PaintContext
// ============================================================================

/// `paint()` 的上下文：提供当前裁剪区域。
///
/// 完整定义见 D3 渲染管线（绘制阶段）。
#[derive(Debug)]
pub struct PaintContext {
    /// 当前裁剪区域。
    pub clip_rect: Rect,
}

impl PaintContext {
    #[must_use]
    pub fn new(clip_rect: Rect) -> Self {
        Self { clip_rect }
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
    fn access_context_empty_focus_path() {
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let ctx = AccessContext::new(bounds);
        assert_eq!(ctx.visible_bounds, bounds);
        assert!(ctx.focus_path.is_empty());
    }
}
