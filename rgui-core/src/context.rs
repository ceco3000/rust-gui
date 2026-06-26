//! Context 类型——WidgetSpec 各方法的上下文参数。
//!
//! 定义源自 D0 §5.5。各 Context 提供 WidgetSpec 方法所需的
//! 只读环境信息或可变操作句柄。

use crate::geometry::{Point, Rect, Size};
use crate::id::WidgetId;
use crate::locale::Locale;
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
    /// 区域设置，包含语言、数字格式、日期格式等本地化信息。
    /// 默认值为 `Locale::EN_US`。
    pub locale: &'static Locale,
}

impl ViewContext {
    /// 创建 ViewContext，locale 默认为 `Locale::EN_US`。
    #[must_use]
    pub const fn new(window_size: Size) -> Self {
        Self {
            window_size,
            locale: Locale::EN_US,
        }
    }

    /// 使用指定 locale 创建 ViewContext。
    #[must_use]
    pub const fn with_locale(window_size: Size, locale: &'static Locale) -> Self {
        Self {
            window_size,
            locale,
        }
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
    /// 当前悬停 widget ID。
    ///
    /// 由框架根据 `MouseEnter`/`MouseLeave` 事件维护。
    /// 组件在 `update()` 中可查询当前悬停状态。
    pub hover: Option<WidgetId>,
    /// 当前事件对应的窗口逻辑坐标。
    pub cursor_window_position: Option<Point>,
    /// 当前事件对应的接收者局部逻辑坐标。
    pub cursor_local_position: Option<Point>,
    /// 当前事件保留的原始平台窗口坐标或自动化原始注入点。
    pub cursor_platform_position: Option<Point>,
}

impl UpdateContext {
    /// 创建空的 UpdateContext（无焦点、无悬停）。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            focus: None,
            hover: None,
            cursor_window_position: None,
            cursor_local_position: None,
            cursor_platform_position: None,
        }
    }

    /// 创建带焦点和悬停信息的 UpdateContext。
    #[must_use]
    pub fn with_focus_and_hover(focus: Option<WidgetId>, hover: Option<WidgetId>) -> Self {
        Self {
            focus,
            hover,
            cursor_window_position: None,
            cursor_local_position: None,
            cursor_platform_position: None,
        }
    }
}

impl Default for UpdateContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FontMetrics
// ============================================================================

/// 单种字体的度量信息（em 单位）。
///
/// 所有值以字体 UPM（Units Per Em）的分数表示。
/// 使用时乘以字体像素大小得到实际像素值。
///
/// 完整定义见 D1 §6.3。
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FontMetrics {
    /// 上升部高度（em 单位）。
    pub ascent: f64,
    /// 下降部高度（em 单位，通常为负值）。
    pub descent: f64,
    /// 行间距（em 单位）。
    pub line_gap: f64,
    /// x-height（em 单位，小写字母 x 的高度）。
    pub x_height: f64,
    /// 大写字母高度（em 单位）。
    pub cap_height: f64,
}

impl FontMetrics {
    /// 创建新的字体度量。
    #[must_use]
    pub const fn new(
        ascent: f64,
        descent: f64,
        line_gap: f64,
        x_height: f64,
        cap_height: f64,
    ) -> Self {
        Self {
            ascent,
            descent,
            line_gap,
            x_height,
            cap_height,
        }
    }

    /// 字体的推荐行高（ascent - descent + line_gap，em 单位）。
    #[must_use]
    pub fn line_height(&self) -> f64 {
        self.ascent - self.descent + self.line_gap
    }
}

// ============================================================================
// FontMetricsCache
// ============================================================================

/// 字体度量缓存。
///
/// 提供常用字体的度量信息，由 rgui-render 的 cosmic-text
/// 集成在启动时填充（见 D1 §6.3 和 D3）。
#[derive(Clone, Debug)]
pub struct FontMetricsCache {
    /// 默认字体族的度量信息。
    pub default_metrics: FontMetrics,
}

impl FontMetricsCache {
    /// 创建新的字体度量缓存。
    #[must_use]
    pub const fn new(default_metrics: FontMetrics) -> Self {
        Self { default_metrics }
    }
}

// ============================================================================
// MeasureContext
// ============================================================================

/// `measure()` 的上下文：提供字体度量和 DPI 信息。
///
/// 完整定义见 D1 §6.3 和 D3 渲染管线（布局阶段）。
#[derive(Clone, Debug)]
pub struct MeasureContext {
    /// 字体度量缓存。
    ///
    /// 提供常用字体的 ascent、descent、line_gap、x_height 等度量信息。
    /// 由 rgui-render 的 cosmic-text 集成提供。
    pub font_metrics: &'static FontMetricsCache,

    /// 当前 DPI 缩放比例。
    pub scale_factor: f64,
}

impl MeasureContext {
    /// 创建新的测量上下文。
    ///
    /// # 参数
    ///
    /// * `font_metrics` — 字体度量缓存静态引用。
    /// * `scale_factor` — 当前 DPI 缩放比例。
    #[must_use]
    pub fn new(font_metrics: &'static FontMetricsCache, scale_factor: f64) -> Self {
        Self {
            font_metrics,
            scale_factor,
        }
    }
}

impl Default for MeasureContext {
    fn default() -> Self {
        Self::new(&FONT_METRICS_CACHE_DEFAULT, 1.0)
    }
}

/// 默认字体度量缓存（Noto Sans CJK SC Regular）。
///
/// 使用实测的 Noto Sans CJK SC Regular 字体度量值。这些值与嵌入的
/// NotoSansCJKsc-Regular.otf 完全匹配。
static FONT_METRICS_CACHE_DEFAULT: FontMetricsCache = FontMetricsCache::new(FontMetrics::new(
    1.160,  // ascent:  1160/1000
    -0.288, // descent: -288/1000
    0.0,    // line_gap: 0/1000
    0.543,  // x_height: 543/1000
    0.733,  // cap_height: 733/1000
));

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
    /// 绘制图像。
    DrawImage {
        /// 目标矩形区域。
        rect: Rect,
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

    /// 提交图像绘制操作。
    ///
    /// `rect` 定义了图像的目标矩形区域（已应用 fit 模式后的最终位置和尺寸）。
    pub fn draw_image(&mut self, rect: Rect) {
        self.operations.push(PaintOp::DrawImage { rect });
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
    fn view_context_default_locale_is_en_us() {
        let ctx = ViewContext::new(Size::new(800.0, 600.0));
        assert_eq!(ctx.locale.id, "en-US");
        assert_eq!(ctx.locale.decimal_separator, '.');
        assert_eq!(ctx.locale.currency_symbol, "$");
    }

    #[test]
    fn view_context_with_locale() {
        let ctx = ViewContext::with_locale(Size::new(800.0, 600.0), Locale::ZH_CN);
        assert_eq!(ctx.locale.id, "zh-CN");
        assert_eq!(ctx.locale.currency_symbol, "¥");
        assert_eq!(ctx.window_size, Size::new(800.0, 600.0));
    }

    #[test]
    fn view_context_clone() {
        let ctx = ViewContext::new(Size::new(100.0, 200.0));
        let cloned = ctx.clone();
        assert_eq!(ctx.window_size, cloned.window_size);
        assert_eq!(ctx.locale.id, cloned.locale.id);
    }

    #[test]
    fn update_context_default_focus_is_none() {
        let ctx = UpdateContext::default();
        assert_eq!(ctx.focus, None);
    }

    #[test]
    fn update_context_default_hover_is_none() {
        let ctx = UpdateContext::default();
        assert_eq!(ctx.hover, None);
    }

    #[test]
    fn update_context_default_cursor_positions_are_none() {
        let ctx = UpdateContext::default();
        assert_eq!(ctx.cursor_window_position, None);
        assert_eq!(ctx.cursor_local_position, None);
        assert_eq!(ctx.cursor_platform_position, None);
    }

    #[test]
    fn update_context_constructs_with_no_focus_no_hover() {
        let ctx = UpdateContext::new();
        assert_eq!(ctx.focus, None);
        assert_eq!(ctx.hover, None);
    }

    #[test]
    fn update_context_constructs_with_focus_and_hover() {
        let widget_id = WidgetId::from_u64(42);
        let ctx = UpdateContext::with_focus_and_hover(Some(widget_id), None);
        assert_eq!(ctx.focus, Some(widget_id));
        assert_eq!(ctx.hover, None);
    }

    #[test]
    fn update_context_hover_set() {
        let widget_id = WidgetId::from_u64(7);
        let ctx = UpdateContext::with_focus_and_hover(None, Some(widget_id));
        assert_eq!(ctx.focus, None);
        assert_eq!(ctx.hover, Some(widget_id));
    }

    #[test]
    fn update_context_both_focus_and_hover() {
        let focus_id = WidgetId::from_u64(1);
        let hover_id = WidgetId::from_u64(2);
        let ctx = UpdateContext::with_focus_and_hover(Some(focus_id), Some(hover_id));
        assert_eq!(ctx.focus, Some(focus_id));
        assert_eq!(ctx.hover, Some(hover_id));
    }

    #[test]
    fn update_context_focus_and_hover_independent() {
        let focus_id = WidgetId::from_u64(10);
        let ctx = UpdateContext::with_focus_and_hover(Some(focus_id), None);
        // 修改 hover 不应影响 focus
        let mut ctx = ctx;
        ctx.hover = Some(WidgetId::from_u64(20));
        assert_eq!(ctx.focus, Some(focus_id));
        assert_eq!(ctx.hover, Some(WidgetId::from_u64(20)));
    }

    #[test]
    fn update_context_debug_output() {
        let id = WidgetId::from_u64(99);
        let ctx = UpdateContext::with_focus_and_hover(Some(id), None);
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("99"), "debug 输出应包含 widget ID: {debug}");
    }

    #[test]
    fn update_context_hover_cleared_on_mouse_leave() {
        let id = WidgetId::from_u64(5);
        let mut ctx = UpdateContext::new();
        // MouseEnter → 设置 hover
        ctx.hover = Some(id);
        assert_eq!(ctx.hover, Some(id));
        // MouseLeave → 清除 hover
        ctx.hover = None;
        assert_eq!(ctx.hover, None);
    }

    #[test]
    fn measure_context_default_scale() {
        let ctx = MeasureContext::default();
        assert!((ctx.scale_factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn measure_context_has_font_metrics() {
        let ctx = MeasureContext::default();
        assert!((ctx.font_metrics.default_metrics.ascent - 1.160).abs() < 0.001);
        assert!((ctx.font_metrics.default_metrics.descent - (-0.288)).abs() < 0.001);
    }

    #[test]
    fn measure_context_custom() {
        static CACHE: FontMetricsCache =
            FontMetricsCache::new(FontMetrics::new(0.8, -0.2, 0.0, 0.5, 0.7));
        let ctx = MeasureContext::new(&CACHE, 2.0);
        assert!((ctx.scale_factor - 2.0).abs() < f64::EPSILON);
        assert!((ctx.font_metrics.default_metrics.ascent - 0.8).abs() < 0.001);
    }

    // ============================================================================
    // FontMetrics 测试
    // ============================================================================

    #[test]
    fn font_metrics_new() {
        let m = FontMetrics::new(0.9, -0.2, 0.05, 0.5, 0.7);
        assert!((m.ascent - 0.9).abs() < 0.001);
        assert!((m.descent - (-0.2)).abs() < 0.001);
        assert!((m.line_gap - 0.05).abs() < 0.001);
        assert!((m.x_height - 0.5).abs() < 0.001);
        assert!((m.cap_height - 0.7).abs() < 0.001);
    }

    #[test]
    fn font_metrics_line_height() {
        let m = FontMetrics::new(0.9, -0.2, 0.05, 0.5, 0.7);
        assert!((m.line_height() - 1.15).abs() < 0.001);
    }

    #[test]
    fn font_metrics_line_height_no_gap() {
        let m = FontMetrics::new(0.8, -0.2, 0.0, 0.5, 0.7);
        assert!((m.line_height() - 1.0).abs() < 0.001);
    }

    #[test]
    fn font_metrics_copy_clone() {
        let m = FontMetrics::new(0.9, -0.2, 0.0, 0.5, 0.7);
        let m2 = m;
        assert_eq!(m, m2);
        let m3 = m.clone();
        assert_eq!(m, m3);
    }

    #[test]
    fn font_metrics_partial_eq() {
        let a = FontMetrics::new(0.9, -0.2, 0.0, 0.5, 0.7);
        let b = FontMetrics::new(0.9, -0.2, 0.0, 0.5, 0.7);
        assert_eq!(a, b);
    }

    #[test]
    fn font_metrics_not_equal() {
        let a = FontMetrics::new(0.9, -0.2, 0.0, 0.5, 0.7);
        let b = FontMetrics::new(0.8, -0.2, 0.0, 0.5, 0.7);
        assert_ne!(a, b);
    }

    // ============================================================================
    // FontMetricsCache 测试
    // ============================================================================

    #[test]
    fn font_metrics_cache_new() {
        let metrics = FontMetrics::new(0.9, -0.2, 0.0, 0.5, 0.7);
        let cache = FontMetricsCache::new(metrics);
        assert_eq!(cache.default_metrics, metrics);
    }

    #[test]
    fn font_metrics_cache_clone() {
        let cache = FontMetricsCache::new(FontMetrics::new(0.9, -0.2, 0.0, 0.5, 0.7));
        let _cloned = cache.clone();
    }

    #[test]
    fn default_font_metrics_cache_is_noto_cjk() {
        let ctx = MeasureContext::default();
        let m = ctx.font_metrics.default_metrics;
        // Noto Sans CJK SC Regular metrics
        assert!((m.ascent - 1.160).abs() < 0.01);
        assert!(m.descent < -0.2);
        assert!(m.x_height > 0.4);
        assert!(m.cap_height > 0.6);
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
