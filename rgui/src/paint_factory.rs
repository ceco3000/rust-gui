//! PaintFn 工厂——根据 WidgetView.props 分发到对应组件的 paint() 方法。
//!
//! 组件通过 `wa-translate` 技能从 Web Awesome (MIT) 手工翻译加入
//! `rgui-components`。翻译后在此添加对应的 match 分支。
//!
//! 布局容器（Container/Row/Column 等）自身不绘制，
//! 子节点的 paint 结果由 walk_view_tree 递归收集。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::traits::AppMessage;
use rgui_core::view::PropValue;
use rgui_render::PaintFn;

use crate::widget_state::WidgetStateStore;

/// 内部实现：PaintFn 工厂（共享 match 体，避免代码重复）。
///
/// 当 `store` 为 `Some` 时，WaAccordionItem 从持久存储读取状态；
/// 为 `None` 时，从 WidgetView.props 创建临时状态。
#[must_use]
fn paint_fn_impl<M: AppMessage>(store: Option<WidgetStateStore>) -> PaintFn<M> {
    fn get_str<'a>(
        props: &'a std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
        key: &str,
    ) -> Option<&'a str> {
        match props.get(key) {
            Some(rgui_core::view::PropValue::Str(s)) => Some(s),
            _ => None,
        }
    }

    Box::new(
        move |view: &rgui_core::view::WidgetView<M>, bounds: Rect| -> Vec<PaintOp> {
            match view.widget_type {
                // ── WA 翻译组件 ──
                "WaAccordion" => {
                    use rgui_components::wa_accordion::{WaAccordion, WaAccordionState};

                    let mut state = WaAccordionState::new();
                    if let Some(m) = get_str(&view.props, "mode") {
                        state.mode = m.to_string();
                    }
                    if let Some(ip) = get_str(&view.props, "icon-placement") {
                        state.icon_placement = ip.to_string();
                    }
                    if let Some(hl) = get_str(&view.props, "heading-level") {
                        state.heading_level = hl.to_string();
                    }
                    if let Some(a) = get_str(&view.props, "appearance") {
                        state.appearance = a.to_string();
                    }
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaAccordion, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "WaAccordionItem" => {
                    use rgui_components::wa_accordion_item::{
                        WaAccordionItem, WaAccordionItemState,
                    };

                    // 若有持久存储则读已有状态；否则每帧从 props 创建
                    let state = if let Some(ref store) = store {
                        let widget_id = view.id.unwrap_or_default();
                        store.get_or_init(widget_id, || {
                            let mut s = WaAccordionItemState::new();
                            if let Some(l) = get_str(&view.props, "label") {
                                s.label = l.to_string();
                            }
                            if let Some(expanded) = view.props.get("expanded") {
                                if let PropValue::Bool(b) = expanded {
                                    s.expanded = *b;
                                }
                            }
                            if let Some(disabled) = view.props.get("disabled") {
                                if let PropValue::Bool(b) = disabled {
                                    s.disabled = *b;
                                }
                            }
                            if let Some(ip) = get_str(&view.props, "icon-placement") {
                                s.icon_placement = ip.to_string();
                            }
                            if let Some(a) = get_str(&view.props, "appearance") {
                                s.appearance = a.to_string();
                            }
                            if let Some(hl) = get_str(&view.props, "heading-level") {
                                s.heading_level = hl.to_string();
                            }
                            if let Some(c) = get_str(&view.props, "content") {
                                s.content = c.to_string();
                            }
                            s
                        })
                    } else {
                        let mut s = WaAccordionItemState::new();
                        if let Some(l) = get_str(&view.props, "label") {
                            s.label = l.to_string();
                        }
                        if let Some(expanded) = view.props.get("expanded") {
                            if let PropValue::Bool(b) = expanded {
                                s.expanded = *b;
                            }
                        }
                        if let Some(disabled) = view.props.get("disabled") {
                            if let PropValue::Bool(b) = disabled {
                                s.disabled = *b;
                            }
                        }
                        if let Some(ip) = get_str(&view.props, "icon-placement") {
                            s.icon_placement = ip.to_string();
                        }
                        if let Some(a) = get_str(&view.props, "appearance") {
                            s.appearance = a.to_string();
                        }
                        if let Some(hl) = get_str(&view.props, "heading-level") {
                            s.heading_level = hl.to_string();
                        }
                        if let Some(c) = get_str(&view.props, "content") {
                            s.content = c.to_string();
                        }
                        s
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(
                        &WaAccordionItem,
                        &state,
                        bounds,
                        &mut ctx,
                    );
                    ctx.into_operations()
                },

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
///
/// 调度到已翻译组件的 paint() 方法。所有组件从 WidgetView.props 创建临时状态。
/// 需要持久交互状态时请使用 [`default_paint_fn_with_state`]。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
    paint_fn_impl(None)
}

/// 创建带实例状态存储的 PaintFn。
///
/// 与 [`default_paint_fn`] 相同，但对于交互式组件（如 WaAccordionItem），
/// 优先从 `store` 读取持久状态，而非每帧从 WidgetView.props 创建临时状态。
/// 这使得组件能够跨帧自主管理交互状态（如展开/折叠）。
#[must_use]
pub fn default_paint_fn_with_state<M: AppMessage>(store: WidgetStateStore) -> PaintFn<M> {
    paint_fn_impl(Some(store))
}

// ============================================================================
// Tier 2 Rhai 脚本执行（T204：加载时一次性执行 paint 脚本，产出 PaintOp 缓存到 props）
// ============================================================================

/// 执行 WidgetView 树中所有 Tier 2 节点的 Rhai paint 脚本。
///
/// 遍历树，对每个 `_tier = "2"` 的节点：
/// 1. 从 `_rhai_path` prop 读取 Rhai 脚本路径
/// 2. 执行脚本——调用 `fill_rect`/`draw_text` 等绘制原语生成 `PaintOp`
/// 3. 将 `PropValue::PaintOps(ops)` 存入 `view.props["paint_ops"]`
///
/// 后续每帧渲染时，`walk_view_tree` 直接从 props 读取预计算的 PaintOp，
/// 无需重新执行脚本（纯 Rust 热路径）。
///
/// # 错误处理
///
/// 脚本执行失败时通过 stderr 报告错误，该节点回退到 `paint_fn`（Tier 1 路径）。
#[cfg(feature = "devtools")]
pub fn execute_tier2_paint_scripts<M: AppMessage>(view: &mut rgui_core::view::WidgetView<M>) {
    execute_tier2_paint_scripts_recursive(view);
}

/// 递归辅助函数。
#[cfg(feature = "devtools")]
fn execute_tier2_paint_scripts_recursive<M: AppMessage>(view: &mut rgui_core::view::WidgetView<M>) {
    // 检查是否为 Tier 2 节点（通过 _tier prop 标识）
    let is_tier2 = view.props.get("_tier").map_or(
        false,
        |v| matches!(v, PropValue::Str(s) if s.as_ref() == "2"),
    );

    if is_tier2 {
        // 读取 Rhai 脚本路径
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

    // 递归处理子节点
    for child in &mut view.children {
        execute_tier2_paint_scripts_recursive(child);
    }
}

/// 执行单个 Rhai paint 脚本并返回生成的 PaintOps。
///
/// 创建独立的 Rhai 引擎，注册绘制原语（`fill_rect`/`draw_text`/`rgb`/`rgba`/`paint_children`），
/// 执行脚本，通过 `PaintOpsAccumulator` 收集结果。
///
/// # 参数
///
/// - `path`: `.rhai` 脚本文件路径
///
/// # 返回
///
/// 脚本执行成功后返回生成的 `Vec<PaintOp>`。失败时返回错误。
#[cfg(feature = "devtools")]
fn execute_rhai_paint_script(
    path: &std::path::Path,
) -> Result<Vec<PaintOp>, Box<dyn std::error::Error>> {
    use rgui_script::ScriptEngine;
    use rgui_script::paint_primitives::{PaintOpsAccumulator, register_paint_primitives};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let script = std::fs::read_to_string(path)?;

    // catch_unwind prevents native function panics from crashing the process
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

    /// Validates that execute_rhai_paint_script returns PaintOps for a valid script.
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

    /// Validates that syntax errors in Rhai scripts return error without crashing.
    #[cfg(feature = "devtools")]
    #[test]
    fn execute_rhai_paint_script_syntax_error_does_not_crash() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("broken.rhai");
        std::fs::write(&rhai_path, "fn broken( {").expect("write rhai file");

        let result = execute_rhai_paint_script(&rhai_path);
        assert!(result.is_err(), "syntax error should return Err");
        let err_msg = result.unwrap_err().to_string();
        // Rhai compilation errors contain position info
        assert!(
            err_msg.contains("error") || err_msg.contains("syntax") || err_msg.contains("parse"),
            "error should mention syntax issue: {err_msg}"
        );
    }

    /// Validates that empty Rhai scripts succeed (return empty PaintOps).
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

    /// Validates that runtime errors in Rhai scripts return error without crashing.
    #[cfg(feature = "devtools")]
    #[test]
    fn execute_rhai_paint_script_runtime_error_does_not_crash() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let rhai_path = dir.path().join("runtime_err.rhai");
        // Call a function that doesn't exist (runtime error)
        std::fs::write(&rhai_path, "nonexistent_fn();").expect("write rhai file");

        let result = execute_rhai_paint_script(&rhai_path);
        assert!(result.is_err(), "runtime error should return Err");
    }
}
