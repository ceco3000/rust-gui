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
use rgui_render::PaintFn;

/// 创建默认的 PaintFn。
///
/// 调度到已翻译组件的 paint() 方法。
/// 未翻译类型返回空 Vec。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
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
                "WaButton" | "Button" => {
                    use rgui_components::wa_button::{WaButton, WaButtonState};
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let state = WaButtonState::new(label);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&WaButton, &state, bounds, &mut ctx);
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
    fn wa_button_paint_produces_ops() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = WidgetView::<TestMsg>::new("WaButton").prop("label", PropValue::str("Click"));
        let ops = paint_fn(&view, Rect::new(0.0, 0.0, 120.0, 40.0));
        assert!(!ops.is_empty(), "WaButton 应产生绘制操作");
    }

    #[test]
    fn unknown_type_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = WidgetView::<TestMsg>::new("Unknown");
        let ops = paint_fn(&view, Rect::new(0.0, 0.0, 100.0, 40.0));
        assert!(ops.is_empty());
    }

    #[test]
    fn button_type_produces_paint_ops() {
        // 验证 "Button" widget_type（.rgui 常用名）也能正确绘制
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = WidgetView::<TestMsg>::new("Button").prop("label", PropValue::str("Click Me"));
        let ops = paint_fn(&view, Rect::new(10.0, 20.0, 120.0, 40.0));
        assert!(!ops.is_empty(), "Button 应产生绘制操作，实际得到 0 个");
        let has_fill = ops.iter().any(|op| matches!(op, PaintOp::FillRect { .. }));
        assert!(
            has_fill,
            "Button 应包含 FillRect（背景），实际 ops: {ops:?}"
        );
        let has_text = ops.iter().any(|op| matches!(op, PaintOp::DrawText { .. }));
        assert!(
            has_text,
            "Button 应包含 DrawText（文字），实际 ops: {ops:?}"
        );
    }
}
