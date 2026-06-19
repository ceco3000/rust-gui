//! PaintFn 工厂——根据 WidgetView.props 分发到对应组件的 paint() 方法。
//!
//! 组件通过 `wa-translate` 技能从 Web Awesome (MIT) 手工翻译。
//! 未翻译组件回退到内置基础绘制（纯色矩形 + 文字）。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::traits::AppMessage;
use rgui_render::PaintFn;

/// 创建默认的 PaintFn。
///
/// 当前支持的组件类型只包含已翻译的组件。
/// 未翻译类型回退到基础绘制。
///
/// 布局容器（Container/Row/Column 等）返回空 Vec——
/// 它们不绘制自身内容，子节点的 paint 结果由 walk_view_tree 递归收集。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
    /// 从 props 中提取字符串属性值。
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
                // ── 已翻译组件 ──
                // (wa_button 等翻译后在此添加 match 分支)

                // ── 内置基础绘制：Button（颜色填充 + 文字）──
                "Button" => {
                    use rgui_core::view::Color;
                    let label = get_str(&view.props, "label").unwrap_or("Button");
                    let bg = Color::new(0.20, 0.50, 0.90, 1.0);
                    let text_color = Color::WHITE;
                    let font_size = bounds.size.height as f32 * 0.8;
                    vec![
                        PaintOp::FillRect {
                            rect: bounds,
                            color: bg,
                            radius: 6.0,
                        },
                        PaintOp::DrawText {
                            text: label.into(),
                            bounds: Rect::new(
                                bounds.origin.x + 4.0,
                                bounds.origin.y,
                                bounds.size.width - 8.0,
                                bounds.size.height,
                            ),
                            color: text_color,
                            font_size,
                        },
                    ]
                },

                // ── 布局容器（自身不绘制）──
                "Container" | "Row" | "Column" | "Padding" | "Center" | "Expanded" | "SizedBox"
                | "Card" | "Stack" | "ScrollView" | "Image" | "ListView" => Vec::new(),

                // ── 未翻译组件 / 未知类型 ──
                _ => Vec::new(),
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

    /// 测试用消息类型。
    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Dummy,
    }

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            match self {
                Self::Dummy => "dummy",
            }
        }
    }

    /// 辅助函数：创建带指定 props 的 WidgetView。
    fn make_view(widget_type: &'static str) -> WidgetView<TestMsg> {
        WidgetView::new(widget_type)
    }

    #[test]
    fn default_paint_fn_creates_valid_paint_fn() {
        let _paint_fn = default_paint_fn::<TestMsg>();
    }

    #[test]
    fn paint_fn_button_extracts_label() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("Button").prop("label", PropValue::str("+1"));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);

        let ops = paint_fn(&view, bounds);
        assert!(!ops.is_empty(), "Button 应产生绘制操作");
        let has_draw_text = ops.iter().any(|op| matches!(op, PaintOp::DrawText { .. }));
        assert!(has_draw_text, "Button 应包含 DrawText");
    }

    #[test]
    fn paint_fn_button_with_default_label() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("Button");
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);

        let ops = paint_fn(&view, bounds);
        assert!(!ops.is_empty(), "无 label 的 Button 也应产生绘制操作");
    }

    #[test]
    fn paint_fn_unknown_type_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("UnknownWidget");
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);

        let ops = paint_fn(&view, bounds);
        assert!(ops.is_empty(), "未知组件类型应返回空 Vec");
    }

    #[test]
    fn paint_fn_container_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("Container");
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);

        let ops = paint_fn(&view, bounds);
        assert!(ops.is_empty(), "容器组件 Container 应返回空 Vec");
    }

    #[test]
    fn paint_fn_row_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("Row");
        let bounds = Rect::new(0.0, 0.0, 400.0, 50.0);

        let ops = paint_fn(&view, bounds);
        assert!(ops.is_empty(), "Row 容器应返回空 Vec");
    }
}
