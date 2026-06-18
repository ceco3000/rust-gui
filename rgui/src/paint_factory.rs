//! PaintFn 工厂——根据 WidgetView.props 提取属性并分发到对应组件的 paint() 方法。
//!
//! 用于 `build_scene_from_view` + `html!` 宏的声明式渲染路径。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::traits::AppMessage;
use rgui_render::PaintFn;

/// 创建默认的 PaintFn，调度到内置组件的 paint() 方法。
///
/// 从 `WidgetView.props` 中提取实际属性（`label`、`text`、`checked`、`value` 等），
/// 构造对应的组件 State，然后调用组件的 `paint()` 方法。
///
/// 支持的组件类型：`Button`、`Label`、`TextField`、`CheckBox`、`Switch`、
/// `Slider`、`ProgressBar`、`Container`、`Row`、`Column`、`Padding`、
/// `Center`、`Expanded`、`SizedBox`、`Card`、`Divider`、`Image`、
/// `ScrollView`、`Stack`。
///
/// 布局容器（Container/Row/Column 等）返回空 Vec——它们不绘制自身内容，
/// 子节点的 paint 结果由 `walk_view_tree` 递归收集。
///
/// 未知组件类型返回空 Vec。
#[must_use]
pub fn default_paint_fn<M: AppMessage>() -> PaintFn<M> {
    /// 从 WidgetView.props 中提取字符串属性值。
    fn get_str<'a>(
        props: &'a std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
        key: &str,
    ) -> Option<&'a str> {
        match props.get(key) {
            Some(rgui_core::view::PropValue::Str(s)) => Some(s),
            _ => None,
        }
    }

    /// 从 WidgetView.props 中提取布尔属性值。
    fn get_bool(
        props: &std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
        key: &str,
    ) -> Option<bool> {
        match props.get(key) {
            Some(rgui_core::view::PropValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// 从 WidgetView.props 中提取 f64 数值（支持 Float 和 Int）。
    fn get_f64(
        props: &std::collections::BTreeMap<&'static str, rgui_core::view::PropValue>,
        key: &str,
    ) -> Option<f64> {
        match props.get(key) {
            Some(rgui_core::view::PropValue::Float(f)) => Some(f.0),
            Some(rgui_core::view::PropValue::Int(i)) => Some(*i as f64),
            _ => None,
        }
    }

    Box::new(
        move |view: &rgui_core::view::WidgetView<M>, bounds: Rect| -> Vec<PaintOp> {
            match view.widget_type {
                "Button" => {
                    use rgui_components::button::{Button, ButtonState};
                    let label = get_str(&view.props, "label").unwrap_or("Button");
                    let state = ButtonState::new(label);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Button, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Label" => {
                    use rgui_components::label::{Label, LabelState};
                    let text = get_str(&view.props, "text").unwrap_or("");
                    let state = LabelState {
                        text: text.to_string(),
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Label, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "TextField" => {
                    use rgui_components::text_field::{TextField, TextFieldState};
                    let text = get_str(&view.props, "text").unwrap_or("");
                    let state = TextFieldState::new(text);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&TextField, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "CheckBox" => {
                    use rgui_components::check_box::{CheckBox, CheckBoxState};
                    let label = get_str(&view.props, "label").unwrap_or("CheckBox");
                    let checked = get_bool(&view.props, "checked").unwrap_or(false);
                    let mut state = CheckBoxState::new(label);
                    state.checked = checked;
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&CheckBox, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Switch" => {
                    use rgui_components::switch::{Switch, SwitchState};
                    let on = get_bool(&view.props, "checked").unwrap_or(false);
                    let label = get_str(&view.props, "label").unwrap_or("");
                    let state = SwitchState {
                        on,
                        label: label.to_string(),
                        ..SwitchState::default()
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Switch, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Slider" => {
                    use rgui_components::slider::{Slider, SliderState};
                    let value = get_f64(&view.props, "value").unwrap_or(50.0);
                    let min = get_f64(&view.props, "min").unwrap_or(0.0);
                    let max = get_f64(&view.props, "max").unwrap_or(100.0);
                    let state = SliderState::new(value, min, max);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Slider, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "ProgressBar" => {
                    use rgui_components::progress_bar::{ProgressBar, ProgressBarState};
                    let value = get_f64(&view.props, "value").unwrap_or(0.5);
                    let label = get_str(&view.props, "label").unwrap_or("").to_string();
                    let mut state = ProgressBarState::new(value);
                    state.label = label;
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&ProgressBar, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "RadioButton" => {
                    use rgui_components::radio_button::{RadioButton, RadioButtonState};
                    let label = get_str(&view.props, "label").unwrap_or("RadioButton");
                    let selected = get_bool(&view.props, "selected").unwrap_or(false);
                    let group = get_str(&view.props, "group").unwrap_or("default");
                    let state = RadioButtonState {
                        label: label.to_string(),
                        selected,
                        group: group.to_string(),
                        ..RadioButtonState::default()
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&RadioButton, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Divider" => {
                    use rgui_components::divider::{Divider, DividerState};
                    let direction_str = get_str(&view.props, "direction");
                    let direction = match direction_str {
                        Some("horizontal") | Some("Horizontal") => {
                            rgui_core::geometry::Axis::Horizontal
                        },
                        _ => rgui_core::geometry::Axis::Vertical,
                    };
                    let state = DividerState {
                        direction,
                        color: None,
                        thickness: None,
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Divider, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                // 布局容器不绘制自身内容，交由子组件绘制
                "Container" | "Row" | "Column" | "Padding" | "Center" | "Expanded" | "SizedBox"
                | "Card" | "Stack" | "ScrollView" | "Image" => Vec::new(),
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
        // 如果能编译，说明类型正确
    }

    #[test]
    fn paint_fn_button_extracts_label() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("Button").prop("label", PropValue::str("+1"));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);

        let ops = paint_fn(&view, bounds);
        // Button 的 paint() 应该至少产生一个 FillRect（背景）和一个 DrawText
        assert!(!ops.is_empty(), "Button 应产生绘制操作");
        // 至少包含 DrawText（标签文本）
        let has_draw_text = ops.iter().any(|op| matches!(op, PaintOp::DrawText { .. }));
        assert!(
            has_draw_text,
            "Button 的绘制操作应包含 DrawText（标签 \"{0}\"）",
            "+1"
        );
    }

    #[test]
    fn paint_fn_button_with_default_label() {
        let paint_fn = default_paint_fn::<TestMsg>();
        // 没有 label prop 的 Button——应使用默认标签
        let view = make_view("Button");
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);

        let ops = paint_fn(&view, bounds);
        assert!(!ops.is_empty(), "无 label 的 Button 也应产生绘制操作");
    }

    #[test]
    fn paint_fn_label_extracts_text() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("Label").prop("text", PropValue::str("计数: 0"));
        let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);

        let ops = paint_fn(&view, bounds);
        assert!(!ops.is_empty(), "Label 应产生绘制操作");
        let has_draw_text = ops.iter().any(|op| matches!(op, PaintOp::DrawText { .. }));
        assert!(has_draw_text, "Label 的绘制操作应包含 DrawText");
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
        assert!(
            ops.is_empty(),
            "容器组件 Container 应返回空 Vec（自身不绘制）"
        );
    }

    #[test]
    fn paint_fn_row_returns_empty() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("Row");
        let bounds = Rect::new(0.0, 0.0, 400.0, 50.0);

        let ops = paint_fn(&view, bounds);
        assert!(ops.is_empty(), "Row 容器应返回空 Vec");
    }

    #[test]
    fn paint_fn_checkbox_extracts_checked() {
        let paint_fn = default_paint_fn::<TestMsg>();
        let view = make_view("CheckBox")
            .prop("label", PropValue::str("同意"))
            .prop("checked", PropValue::Bool(true));
        let bounds = Rect::new(0.0, 0.0, 150.0, 30.0);

        let ops = paint_fn(&view, bounds);
        assert!(!ops.is_empty(), "CheckBox 应产生绘制操作");
    }
}
