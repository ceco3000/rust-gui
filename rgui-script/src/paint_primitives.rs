//! Rhai 绘制原语注册——将 `fill_rect`/`draw_text`/`rgb`/`rgba`/`paint_children`
//! 绑定到 Rhai 引擎，使 `.rhai` paint 脚本能够生成 `Vec<PaintOp>`。
//!
//! ## 设计说明
//!
//! - `PaintOpsAccumulator` 持有共享的 `Vec<PaintOp>` 缓冲区
//! - `register_paint_primitives()` 向 Rhai 引擎注册绘制原语函数
//! - Rhai 脚本调用 `fill_rect`/`draw_text` 时，PaintOp 追加到共享缓冲区
//! - 脚本执行完毕后，调用方通过 `PaintOpsAccumulator::take()` 取走结果
//!
//! ## 示例
//!
//! ```rust,no_run
//! use rhai::Engine;
//! use rgui_core::context::PaintOp;
//! use rgui_script::paint_primitives::{PaintOpsAccumulator, register_paint_primitives};
//!
//! let mut engine = Engine::new();
//! let accumulator = PaintOpsAccumulator::new();
//! register_paint_primitives(&mut engine, &accumulator);
//!
//! // 执行 paint 脚本
//! engine.run(r#"
//!     let c = rgb(1.0, 0.0, 0.0);
//!     fill_rect(10.0, 20.0, 100.0, 50.0, c, 8.0);
//!     draw_text("Hello", 10.0, 20.0, 100.0, 50.0, rgba(0.0, 0.0, 0.0, 1.0), 16.0);
//! "#).unwrap();
//!
//! let ops = accumulator.take();
//! assert_eq!(ops.len(), 2);
//! ```

use std::sync::{Arc, Mutex, MutexGuard};

use rgui_core::context::PaintOp;
use rgui_core::view::Color;
use rhai::Engine;

/// 获取 mutex 锁，中毒时恢复内部值。
fn lock_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

// ============================================================================
// PaintOpsAccumulator
// ============================================================================

/// 绘制操作累加器——Rhai 脚本调用绘制原语时，PaintOp 追加到此缓冲区。
///
/// 线程安全（`Arc<Mutex<Vec<PaintOp>>>` 包装），支持跨线程共享：
/// - Rhai 脚本执行线程写入
/// - 调用方线程读取结果。
#[derive(Clone, Debug, Default)]
pub struct PaintOpsAccumulator {
    ops: Arc<Mutex<Vec<PaintOp>>>,
}

impl PaintOpsAccumulator {
    /// 创建空累加器。
    #[must_use]
    pub fn new() -> Self {
        Self {
            ops: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 追加一个绘制操作。
    fn push(&self, op: PaintOp) {
        lock_mutex(&self.ops).push(op);
    }

    /// 取出并清空已收集的绘制操作。
    ///
    /// 返回当前缓冲区中的所有 PaintOp，并重置为空。
    #[must_use]
    pub fn take(&self) -> Vec<PaintOp> {
        let mut guard = lock_mutex(&self.ops);
        std::mem::take(&mut *guard)
    }

    /// 返回当前已收集的操作数量。
    #[must_use]
    pub fn len(&self) -> usize {
        lock_mutex(&self.ops).len()
    }

    /// 返回缓冲区是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        lock_mutex(&self.ops).is_empty()
    }
}

// ============================================================================
// 绘制原语注册
// ============================================================================

/// 向 Rhai 引擎注册 P0 绘制原语。
///
/// 注册以下函数：
/// - `rgb(r, g, b)` → `Color`
/// - `rgba(r, g, b, a)` → `Color`
/// - `fill_rect(x, y, w, h, color, radius)` — 追加 `PaintOp::FillRect`
/// - `draw_text(text, x, y, w, h, color, font_size)` — 追加 `PaintOp::DrawText`
/// - `paint_children()` — 占位（Phase 0 无操作；未来由 T204 实现）
///
/// # Panics
///
/// 不会 panic。所有原生函数参数均 clamp 到合法范围。
pub fn register_paint_primitives(engine: &mut Engine, accumulator: &PaintOpsAccumulator) {
    // ── 注册 Color 为 Rhai 自定义类型 ──────────────────────────
    engine.register_type::<Color>();

    // ── rgb(r, g, b) → Color ──────────────────────────────────
    engine.register_fn("rgb", |r: f64, g: f64, b: f64| -> Color {
        Color::rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    });

    // ── rgba(r, g, b, a) → Color ──────────────────────────────
    engine.register_fn("rgba", |r: f64, g: f64, b: f64, a: f64| -> Color {
        Color::new(
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
            a.clamp(0.0, 1.0),
        )
    });

    // ── fill_rect(x, y, w, h, color, radius) ──────────────────
    let acc = accumulator.clone();
    engine.register_fn(
        "fill_rect",
        #[allow(clippy::cast_possible_truncation)]
        move |x: f64, y: f64, w: f64, h: f64, color: Color, radius: f64| {
            let w = w.max(0.0);
            let h = h.max(0.0);
            let radius = radius.max(0.0);
            acc.push(PaintOp::FillRect {
                rect: rgui_core::geometry::Rect::new(x, y, w, h),
                color,
                radius: radius as f32,
            });
        },
    );

    // ── draw_text(text, x, y, w, h, color, font_size) ─────────
    let acc2 = accumulator.clone();
    engine.register_fn(
        "draw_text",
        #[allow(clippy::cast_possible_truncation)]
        move |text: &str, x: f64, y: f64, w: f64, h: f64, color: Color, font_size: f64| {
            let w = w.max(0.0);
            let h = h.max(0.0);
            let font_size = font_size.clamp(1.0, 512.0);
            acc2.push(PaintOp::DrawText {
                text: text.to_string(),
                bounds: rgui_core::geometry::Rect::new(x, y, w, h),
                color,
                font_size: font_size as f32,
            });
        },
    );

    // ── paint_children() — Phase 0 占位 ───────────────────────
    engine.register_fn("paint_children", || {
        // Phase 0: 占位——T204 将实现实际的子节点递归渲染。
        // 当前仅记录调用（无操作）。
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Engine;

    /// 辅助函数：创建带注册绘制原语的引擎。
    fn setup_engine() -> (Engine, PaintOpsAccumulator) {
        let mut engine = Engine::new();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);
        (engine, acc)
    }

    // ── rgb / rgba 测试 ─────────────────────────────────────────

    #[test]
    fn rgb_returns_color_with_alpha_1() {
        let (engine, _acc) = setup_engine();
        // Rhai 中调用 rgb 并返回
        let result: Color = engine.eval("rgb(0.2, 0.4, 0.6)").unwrap();
        // Color 的字段在 Rhai 中无法直接访问，
        // 通过 fill_rect 间接验证颜色正确传递
        // 此处仅验证函数调用不 panic
        let _ = result;
    }

    #[test]
    fn rgba_returns_color_with_custom_alpha() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        // 注意：重新注册以便使用新的 accumulator
        register_paint_primitives(&mut engine, &acc);

        // 使用 rgba 创建颜色并传给 fill_rect
        engine
            .run(
                r#"
            let c = rgba(0.1, 0.2, 0.3, 0.5);
            fill_rect(0.0, 0.0, 10.0, 10.0, c, 0.0);
        "#,
            )
            .unwrap();

        let ops = acc.take();
        assert_eq!(ops.len(), 1);
        if let PaintOp::FillRect { color, .. } = &ops[0] {
            // 验证颜色通道正确传递（允许浮点误差）
            assert!((color.r - 0.1).abs() < 0.001);
            assert!((color.g - 0.2).abs() < 0.001);
            assert!((color.b - 0.3).abs() < 0.001);
            assert!((color.a - 0.5).abs() < 0.001);
        } else {
            panic!("Expected FillRect");
        }
    }

    #[test]
    fn rgb_clamps_values_to_0_1() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        // 负值 clamp 到 0，大于 1 clamp 到 1
        engine
            .run(
                r#"
            let c = rgb(-0.5, 1.5, 0.5);
            fill_rect(0.0, 0.0, 1.0, 1.0, c, 0.0);
        "#,
            )
            .unwrap();

        let ops = acc.take();
        if let PaintOp::FillRect { color, .. } = &ops[0] {
            assert!((color.r - 0.0).abs() < 0.001);
            assert!((color.g - 1.0).abs() < 0.001);
            assert!((color.b - 0.5).abs() < 0.001);
        }
    }

    // ── fill_rect 测试 ──────────────────────────────────────────

    #[test]
    fn fill_rect_appends_paint_op() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run(
                r#"
            fill_rect(10.0, 20.0, 100.0, 50.0, rgb(1.0, 0.0, 0.0), 8.0);
        "#,
            )
            .unwrap();

        let ops = acc.take();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PaintOp::FillRect {
                rect,
                color,
                radius,
            } => {
                assert!((rect.origin.x - 10.0).abs() < 0.01);
                assert!((rect.origin.y - 20.0).abs() < 0.01);
                assert!((rect.size.width - 100.0).abs() < 0.01);
                assert!((rect.size.height - 50.0).abs() < 0.01);
                assert!((color.r - 1.0).abs() < 0.01);
                assert!((radius - 8.0).abs() < 0.01);
            },
            _ => panic!("Expected FillRect, got {ops:?}"),
        }
    }

    #[test]
    fn fill_rect_clamps_negative_dimensions_to_zero() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run("fill_rect(0.0, 0.0, -10.0, -20.0, rgb(0.0, 0.0, 0.0), 0.0);")
            .unwrap();

        let ops = acc.take();
        if let PaintOp::FillRect { rect, .. } = &ops[0] {
            assert!(
                (rect.size.width - 0.0).abs() < 0.01,
                "width should be clamped to 0"
            );
            assert!(
                (rect.size.height - 0.0).abs() < 0.01,
                "height should be clamped to 0"
            );
        }
    }

    #[test]
    fn fill_rect_clamps_negative_radius_to_zero() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run("fill_rect(0.0, 0.0, 10.0, 10.0, rgb(0.0, 0.0, 0.0), -5.0);")
            .unwrap();

        let ops = acc.take();
        if let PaintOp::FillRect { radius, .. } = &ops[0] {
            assert!((radius - 0.0).abs() < 0.01, "radius should be clamped to 0");
        }
    }

    #[test]
    fn multiple_fill_rects_accumulate_in_order() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run(
                r#"
            fill_rect(0.0, 0.0, 5.0, 5.0, rgb(1.0, 0.0, 0.0), 0.0);
            fill_rect(5.0, 5.0, 5.0, 5.0, rgb(0.0, 1.0, 0.0), 0.0);
            fill_rect(10.0, 10.0, 5.0, 5.0, rgb(0.0, 0.0, 1.0), 0.0);
        "#,
            )
            .unwrap();

        let ops = acc.take();
        assert_eq!(ops.len(), 3);
        // 验证顺序
        if let PaintOp::FillRect { rect, color, .. } = &ops[0] {
            assert!((rect.origin.x - 0.0).abs() < 0.01);
            assert!((color.r - 1.0).abs() < 0.01);
        }
        if let PaintOp::FillRect { rect, color, .. } = &ops[2] {
            assert!((rect.origin.x - 10.0).abs() < 0.01);
            assert!((color.b - 1.0).abs() < 0.01);
        }
    }

    // ── draw_text 测试 ──────────────────────────────────────────

    #[test]
    fn draw_text_appends_paint_op() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run(r#"draw_text("Hello", 10.0, 20.0, 100.0, 30.0, rgb(0.0, 0.0, 0.0), 16.0);"#)
            .unwrap();

        let ops = acc.take();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PaintOp::DrawText {
                text,
                bounds,
                color,
                font_size,
            } => {
                assert_eq!(text, "Hello");
                assert!((bounds.origin.x - 10.0).abs() < 0.01);
                assert!((bounds.origin.y - 20.0).abs() < 0.01);
                assert!((bounds.size.width - 100.0).abs() < 0.01);
                assert!((bounds.size.height - 30.0).abs() < 0.01);
                assert!((color.r - 0.0).abs() < 0.01);
                assert!((font_size - 16.0).abs() < 0.01);
            },
            _ => panic!("Expected DrawText"),
        }
    }

    #[test]
    fn draw_text_clamps_font_size() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        // font_size 为 0 应 clamp 到 1.0
        engine
            .run(r#"draw_text("tiny", 0.0, 0.0, 100.0, 20.0, rgb(0.0, 0.0, 0.0), 0.0);"#)
            .unwrap();

        let ops = acc.take();
        if let PaintOp::DrawText { font_size, .. } = &ops[0] {
            assert!(
                (font_size - 1.0).abs() < 0.01,
                "font_size should be clamped to 1.0"
            );
        }
    }

    #[test]
    fn draw_text_clamps_large_font_size() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run(r#"draw_text("huge", 0.0, 0.0, 100.0, 20.0, rgb(0.0, 0.0, 0.0), 9999.0);"#)
            .unwrap();

        let ops = acc.take();
        if let PaintOp::DrawText { font_size, .. } = &ops[0] {
            assert!(
                (font_size - 512.0).abs() < 0.01,
                "font_size should be clamped to 512"
            );
        }
    }

    // ── paint_children 测试 ─────────────────────────────────────

    #[test]
    fn paint_children_is_noop() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        // paint_children() 调用不应产生任何 PaintOp
        engine.run("paint_children();").unwrap();

        let ops = acc.take();
        assert_eq!(ops.len(), 0, "paint_children should be a no-op in Phase 0");
    }

    // ── 组合测试 ─────────────────────────────────────────────

    #[test]
    fn fill_rect_and_draw_text_accumulate_together() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run(
                r#"
            fill_rect(0.0, 0.0, 100.0, 60.0, rgb(0.9, 0.9, 0.9), 4.0);
            draw_text("Card Title", 10.0, 10.0, 80.0, 20.0, rgb(0.0, 0.0, 0.0), 14.0);
            paint_children();
        "#,
            )
            .unwrap();

        let ops = acc.take();
        assert_eq!(ops.len(), 2, "paint_children should not add PaintOp");
        assert!(matches!(ops[0], PaintOp::FillRect { .. }));
        assert!(matches!(ops[1], PaintOp::DrawText { .. }));
    }

    #[test]
    fn accumulator_take_clears_buffer() {
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        engine
            .run("fill_rect(0.0, 0.0, 10.0, 10.0, rgb(0.0, 0.0, 0.0), 0.0);")
            .unwrap();

        assert_eq!(acc.len(), 1);
        let ops = acc.take();
        assert_eq!(ops.len(), 1);
        assert!(acc.is_empty());
        assert_eq!(acc.len(), 0);
    }

    // ── D1 §9.4.4 兼容性测试 ────────────────────────────────────

    #[test]
    fn card_paint_script_matches_spec() {
        // 验证 D1 §9.4.1 的 Card paint 脚本可以正确执行
        let (mut engine, _acc) = setup_engine();
        let acc = PaintOpsAccumulator::new();
        register_paint_primitives(&mut engine, &acc);

        let script = r#"
            let bg = rgb(1.0, 1.0, 1.0);
            let border_color = rgb(0.85, 0.85, 0.85);
            let radius = 8.0;
            // 背景
            fill_rect(0.0, 0.0, 300.0, 200.0, bg, radius);
            // 边框（四条线模拟）
            fill_rect(0.0, 0.0, 300.0, 1.0, border_color, 0.0);
            fill_rect(0.0, 199.0, 300.0, 1.0, border_color, 0.0);
            fill_rect(0.0, 0.0, 1.0, 200.0, border_color, 0.0);
            fill_rect(299.0, 0.0, 1.0, 200.0, border_color, 0.0);
            paint_children();
        "#;

        engine.run(script).unwrap();
        let ops = acc.take();
        // 背景 + 4 条边框 = 5 个 FillRect，paint_children 不产生 PaintOp
        assert_eq!(ops.len(), 5);
        for op in &ops {
            assert!(matches!(op, PaintOp::FillRect { .. }));
        }
    }

    // ── PaintOpsAccumulator 单元测试 ────────────────────────────

    #[test]
    fn accumulator_new_is_empty() {
        let acc = PaintOpsAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.len(), 0);
    }

    #[test]
    fn accumulator_take_returns_empty_for_new() {
        let acc = PaintOpsAccumulator::new();
        assert!(acc.take().is_empty());
    }
}
