//! PaintFn 工厂——根据 WidgetView.props 分发到对应组件的 paint() 方法。
//!
//! 布局容器（Container/Row/Column 等）自身不绘制，
//! 子节点的 paint 结果由 walk_view_tree 递归收集。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::traits::AppMessage;
use rgui_render::PaintFn;

use crate::widget_state::WidgetStateStore;

/// 内部实现：PaintFn 工厂（共享 match 体，避免代码重复）。
#[must_use]
fn paint_fn_impl<M: AppMessage>(_store: Option<WidgetStateStore>) -> PaintFn<M> {
    Box::new(
        move |view: &rgui_core::view::WidgetView<M>, _bounds: Rect| -> Vec<PaintOp> {
            match view.widget_type {
                // ── 布局容器（自身不绘制）──
                "Container" | "Row" | "Column" | "Padding" | "Center" | "Expanded" | "SizedBox"
                | "Card" | "Stack" | "ScrollView" | "ListView" => Vec::new(),

                // ── 未翻译 / 未知 ──
                unknown => {
                    eprintln!(
                        "[rgui] paint_factory: 未知 widget_type=\"{unknown}\"，无 paint 实现，返回空"
                    );
                    Vec::new()
                },
            }
        },
    )
}

/// 创建默认的 PaintFn（无持久状态存储）。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
    paint_fn_impl(None)
}

/// 创建带实例状态存储的 PaintFn。
///
/// 与 [`default_paint_fn`] 相同，当前组件库为空，预留接口。
#[must_use]
pub fn default_paint_fn_with_state<M: AppMessage>(store: WidgetStateStore) -> PaintFn<M> {
    paint_fn_impl(Some(store))
}

// ============================================================================
// Tier 2 Rhai 脚本执行（T204：加载时一次性执行 paint 脚本，产出 PaintOp 缓存到 props）
// ============================================================================

/// 执行 WidgetView 树中所有 Tier 2 节点的 Rhai paint 脚本。
#[cfg(feature = "devtools")]
pub fn execute_tier2_paint_scripts<M: AppMessage>(view: &mut rgui_core::view::WidgetView<M>) {
    execute_tier2_paint_scripts_recursive(view);
}

/// 递归辅助函数。
#[cfg(feature = "devtools")]
fn execute_tier2_paint_scripts_recursive<M: AppMessage>(view: &mut rgui_core::view::WidgetView<M>) {
    use rgui_core::view::PropValue;

    let is_tier2 = view.props.get("_tier").map_or(
        false,
        |v| matches!(v, PropValue::Str(s) if s.as_ref() == "2"),
    );

    if is_tier2 {
        let rhai_path = view.props.get("_rhai_path").and_then(|v| match v {
            PropValue::Str(s) => Some(std::path::PathBuf::from(s.as_ref())),
            _ => None,
        });

        if let Some(rhai_path) = rhai_path {
            match execute_rhai_paint_script(&rhai_path) {
                Ok(ops) => {
                    view.props.insert("paint_ops", PropValue::PaintOps(ops));
                },
                Err(e) => {
                    eprintln!(
                        "[rgui] execute_tier2_paint_scripts: Rhai 脚本执行失败 ({}): {e}",
                        rhai_path.display()
                    );
                },
            }
        }
    }

    for child in &mut view.children {
        execute_tier2_paint_scripts_recursive(child);
    }
}

/// 执行单个 Rhai paint 脚本并返回生成的 PaintOps。
#[cfg(feature = "devtools")]
fn execute_rhai_paint_script(
    path: &std::path::Path,
) -> Result<Vec<PaintOp>, Box<dyn std::error::Error>> {
    use rgui_script::ScriptEngine;
    use rgui_script::paint_primitives::{PaintOpsAccumulator, register_paint_primitives};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let script = std::fs::read_to_string(path)?;

    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut engine = ScriptEngine::new();
        let accumulator = PaintOpsAccumulator::new();
        register_paint_primitives(engine.engine_mut(), &accumulator);

        engine.run(&script)?;

        Ok::<Vec<PaintOp>, Box<dyn std::error::Error>>(accumulator.take())
    }));

    match result {
        Ok(Ok(ops)) => Ok(ops),
        Ok(Err(e)) => Err(e),
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            Err(format!("Rhai paint script panic ({}): {msg}", path.display()).into())
        },
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rgui_core::geometry::Rect;
    use rgui_core::view::{PropValue, WidgetView};

    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Dummy,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            "dummy"
        }
    }

    #[test]
    fn default_paint_fn_creates_valid_paint_fn() {
        let _paint_fn = default_paint_fn::<TestMsg>();
    }

    #[test]
    fn unknown_type_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = WidgetView::<TestMsg>::new("Unknown");
        let ops = paint_fn(&view, Rect::new(0.0, 0.0, 100.0, 40.0));
        assert!(ops.is_empty());
    }

    // ── T206: Tier 2 paint script error handling ──────────────────────

    #[cfg(feature = "devtools")]
    #[test]
    fn execute_rhai_paint_script_valid_returns_ops() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("paint.rhai");
        std::fs::write(
            &rhai_path,
            r#"fill_rect(0.0, 0.0, 100.0, 50.0, rgb(0.5, 0.5, 0.5), 4.0);"#,
        )
        .expect("write rhai file");

        let result = execute_rhai_paint_script(&rhai_path);
        assert!(
            result.is_ok(),
            "valid script should succeed: {:?}",
            result.err()
        );
        let ops = result.unwrap();
        assert_eq!(ops.len(), 1, "should produce 1 PaintOp");
        assert!(matches!(ops[0], PaintOp::FillRect { .. }));
    }

    #[cfg(feature = "devtools")]
    #[test]
    fn execute_rhai_paint_script_syntax_error_does_not_crash() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("broken.rhai");
        std::fs::write(&rhai_path, "fn broken( {").expect("write rhai file");

        let result = execute_rhai_paint_script(&rhai_path);
        assert!(result.is_err(), "syntax error should return Err");
    }

    #[cfg(feature = "devtools")]
    #[test]
    fn execute_rhai_paint_script_empty_script() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("empty.rhai");
        std::fs::write(&rhai_path, "").expect("write rhai file");

        let result = execute_rhai_paint_script(&rhai_path);
        assert!(result.is_ok(), "empty script should succeed");
        let ops = result.unwrap();
        assert!(ops.is_empty(), "empty script should produce no ops");
    }

    #[cfg(feature = "devtools")]
    #[test]
    fn execute_rhai_paint_script_runtime_error_does_not_crash() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("runtime_err.rhai");
        std::fs::write(&rhai_path, "nonexistent_fn();").expect("write rhai file");

        let result = execute_rhai_paint_script(&rhai_path);
        assert!(result.is_err(), "runtime error should return Err");
    }
}
