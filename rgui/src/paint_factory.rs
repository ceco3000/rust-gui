//! PaintFn 工厂——根据 WidgetView.props 分发到对应组件的 paint() 方法。
//!
//! 布局容器（Container/Row/Column 等）自身不绘制，
//! 子节点的 paint 结果由 walk_view_tree 递归收集。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::traits::{AppMessage, WidgetSpec};
use rgui_render::PaintFn;

use rgui_components::wa_badge::{WaBadge, WaBadgeState};
use rgui_core::view::PropValue;
use rgui_core::widget_state::WidgetStateStore;

/// 内部实现：PaintFn 工厂（共享 match 体，避免代码重复）。
#[must_use]
fn paint_fn_impl<M: AppMessage>(_store: Option<WidgetStateStore>) -> PaintFn<M> {
    Box::new(
        move |view: &rgui_core::view::WidgetView<M>, _bounds: Rect| -> Vec<PaintOp> {
            match view.widget_type {
                // ── 布局容器（自身不绘制）──
                "Container" | "Row" | "Column" | "Padding" | "Center" | "Expanded" | "SizedBox"
                | "Card" | "Stack" | "ScrollView" | "ListView" => Vec::new(),

                // ── Tier 1 WidgetSpec 组件 ──
                "WaBadge" => {
                    let label = view
                        .props
                        .get("label")
                        .and_then(|v| match v {
                            PropValue::Str(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let variant = view
                        .props
                        .get("variant")
                        .and_then(|v| match v {
                            PropValue::Str(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| String::from("brand"));
                    let appearance = view
                        .props
                        .get("appearance")
                        .and_then(|v| match v {
                            PropValue::Str(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| String::from("accent"));
                    let pill = view
                        .props
                        .get("pill")
                        .and_then(|v| match v {
                            PropValue::Bool(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(false);
                    let attention = view
                        .props
                        .get("attention")
                        .and_then(|v| match v {
                            PropValue::Str(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .unwrap_or_else(|| String::from("none"));

                    let state = WaBadgeState {
                        label,
                        variant,
                        appearance,
                        pill,
                        attention,
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(_bounds);
                    WaBadge.paint(&state, _bounds, &mut ctx);
                    ctx.into_operations()
                },

                // ── 未翻译 / 未知 ──
                unknown => {
                    log::warn!(target: "rgui::core",
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
///
/// `layout_engine` 提供每个 widget 的布局 bounds，
/// 注入为 Rhai 作用域的 `width`/`height` 变量（AC02）。
///
/// **AC07：** 此函数每次调用都会重新执行所有 Tier 2 节点的 paint 脚本。
/// 调用方负责在 props 变更后调用本函数以刷新 `paint_ops` 缓存。
#[cfg(feature = "devtools")]
pub fn execute_tier2_paint_scripts<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    layout_engine: &rgui_layout::LayoutEngine,
) {
    execute_tier2_paint_scripts_recursive(view, layout_engine);
}

/// 递归辅助函数。
///
/// C8: 脚本执行失败时，从 _old_paint_ops 恢复旧的 paint_ops 缓存作为降级方案。
#[cfg(feature = "devtools")]
fn execute_tier2_paint_scripts_recursive<M: AppMessage>(
    view: &mut rgui_core::view::WidgetView<M>,
    layout_engine: &rgui_layout::LayoutEngine,
) {
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
            // AC02: 从布局引擎获取 bounds，注入为 width/height
            let (width, height) = view
                .id
                .and_then(|id| {
                    layout_engine
                        .get_layout(id)
                        .map(|cached| (cached.result.size.width, cached.result.size.height))
                })
                .unwrap_or((400.0, 300.0));

            // AC07: 传递 view.props 以注入 expanded/label/content 等 prop 变量
            match execute_rhai_paint_script(&rhai_path, width, height, &view.props) {
                Ok(ops) => {
                    view.props.insert("paint_ops", PropValue::PaintOps(ops));
                    // C8: 成功执行后清理 _old_paint_ops
                    view.props.remove("_old_paint_ops");
                },
                Err(e) => {
                    log::error!(target: "rgui::script",
                        "[rgui] execute_tier2_paint_scripts: Rhai 脚本执行失败 ({}): {e}",
                        rhai_path.display()
                    );
                    // C8: 恢复旧的 paint_ops 缓存作为降级方案
                    if let Some(old_ops) = view.props.remove("_old_paint_ops") {
                        view.props.insert("paint_ops", old_ops);
                        log::warn!(target: "rgui::script",
                            "[rgui] 降级：使用旧的 paint_ops 缓存 ({})", rhai_path.display());
                    }
                },
            }
        }
    }

    for child in &mut view.children {
        execute_tier2_paint_scripts_recursive(child, layout_engine);
    }
}

/// 执行单个 Rhai paint 脚本并返回生成的 PaintOps。
///
/// `width` 和 `height` 被注入为 Rhai 作用域变量，
/// 脚本可通过 `fill_rect(0.0, 0.0, width, height, ...)` 使用。
/// 这使 paint 脚本能自适应组件的布局 bounds（AC02）。
#[cfg(feature = "devtools")]
fn execute_rhai_paint_script(
    path: &std::path::Path,
    width: f64,
    height: f64,
    props: &std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
) -> Result<Vec<PaintOp>, Box<dyn std::error::Error>> {
    use rgui_script::ScriptEngine;
    use rgui_script::paint_primitives::{PaintOpsAccumulator, register_paint_primitives};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let script = std::fs::read_to_string(path)?;

    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut engine = ScriptEngine::new();
        let accumulator = PaintOpsAccumulator::new();
        register_paint_primitives(engine.engine_mut(), &accumulator);

        // AC02: 注入 width/height 变量到 Rhai 作用域
        // 使用 new_static_scope()（'static 生命周期）避免与
        // engine.engine_mut()（register_paint_primitives 内部调用）的借用冲突。
        let mut scope = ScriptEngine::new_static_scope();
        scope.push("width", width);
        scope.push("height", height);

        // AC07: 注入 widget props 为 Rhai 变量（expanded, label, content, disabled 等）
        for (key, value) in props {
            // 跳过内部元数据 props（以 _ 开头）
            if key.starts_with('_') {
                continue;
            }
            match value {
                rgui_core::view::PropValue::Str(s) => {
                    scope.push(key.to_string(), s.to_string());
                },
                rgui_core::view::PropValue::Bool(b) => {
                    scope.push(key.to_string(), *b);
                },
                rgui_core::view::PropValue::Int(i) => {
                    scope.push(key.to_string(), *i);
                },
                rgui_core::view::PropValue::Float(f) => {
                    scope.push(key.to_string(), f.0);
                },
                _ => {
                    // 跳过复杂类型（Color, Size, Rect, List, Map, Enum, Callback, PaintOps）
                },
            }
        }

        engine.run_with_scope(&mut scope, &script)?;

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

        let result =
            execute_rhai_paint_script(&rhai_path, 100.0, 50.0, &std::collections::BTreeMap::new());
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

        let result =
            execute_rhai_paint_script(&rhai_path, 100.0, 50.0, &std::collections::BTreeMap::new());
        assert!(result.is_err(), "syntax error should return Err");
    }

    #[cfg(feature = "devtools")]
    #[test]
    fn execute_rhai_paint_script_empty_script() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("empty.rhai");
        std::fs::write(&rhai_path, "").expect("write rhai file");

        let result =
            execute_rhai_paint_script(&rhai_path, 100.0, 50.0, &std::collections::BTreeMap::new());
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

        let result =
            execute_rhai_paint_script(&rhai_path, 400.0, 300.0, &std::collections::BTreeMap::new());
        assert!(result.is_err(), "runtime error should return Err");
    }

    // ── AC02: Tier 2 paint script bounds injection ──────────────────

    #[cfg(feature = "devtools")]
    #[test]
    fn ac02_bounds_injected_as_width_height() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("bounds_test.rhai");
        std::fs::write(
            &rhai_path,
            r#"fill_rect(0.0, 0.0, width, height, rgb(0.5, 0.5, 0.5), 4.0);"#,
        )
        .expect("write rhai file");

        let result =
            execute_rhai_paint_script(&rhai_path, 400.0, 300.0, &std::collections::BTreeMap::new());
        assert!(
            result.is_ok(),
            "script using width/height should succeed: {:?}",
            result.err()
        );
        let ops = result.unwrap();
        assert_eq!(ops.len(), 1, "should produce 1 PaintOp");
        assert!(matches!(ops[0], PaintOp::FillRect { .. }));
        // Verify the rect uses the injected bounds
        if let PaintOp::FillRect { rect, .. } = &ops[0] {
            assert!(
                (rect.size.width - 400.0).abs() < 0.01,
                "width should be 400.0, got {}",
                rect.size.width
            );
            assert!(
                (rect.size.height - 300.0).abs() < 0.01,
                "height should be 300.0, got {}",
                rect.size.height
            );
        }
    }

    #[cfg(feature = "devtools")]
    #[test]
    fn ac02_bounds_script_without_width_height_still_works() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("no_bounds.rhai");
        std::fs::write(
            &rhai_path,
            r#"fill_rect(10.0, 20.0, 100.0, 50.0, rgb(0.1, 0.2, 0.3), 0.0);"#,
        )
        .expect("write rhai file");

        let result =
            execute_rhai_paint_script(&rhai_path, 400.0, 300.0, &std::collections::BTreeMap::new());
        assert!(
            result.is_ok(),
            "script without width/height refs should still work"
        );
    }

    #[cfg(feature = "devtools")]
    #[test]
    fn ac02_injected_variables_are_accessible_in_expressions() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("expr_test.rhai");
        std::fs::write(
            &rhai_path,
            r#"let half_w = width / 2.0; fill_rect(half_w - 10.0, 0.0, 20.0, height, rgb(0.0, 0.0, 1.0), 0.0);"#,
        )
        .expect("write rhai file");

        let result =
            execute_rhai_paint_script(&rhai_path, 400.0, 300.0, &std::collections::BTreeMap::new());
        assert!(
            result.is_ok(),
            "script using width/height in expressions should succeed"
        );
        let ops = result.unwrap();
        assert_eq!(ops.len(), 1, "should produce 1 PaintOp");
    }
}
