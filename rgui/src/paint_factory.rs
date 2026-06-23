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
                    rgui_core::traits::WidgetSpec::paint(
                        &WaAccordion,
                        &state,
                        bounds,
                        &mut ctx,
                    );
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
}
