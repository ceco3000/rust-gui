//! PaintFn 工厂——根据 widget 类型名分发到对应组件的 paint() 方法。
//!
//! 用于 `build_scene_from_view` + `html!` 宏的声明式渲染路径。

use rgui_core::context::PaintOp;
use rgui_core::geometry::Rect;
use rgui_core::id::WidgetId;
use rgui_render::PaintFn;

/// 创建默认的 PaintFn，调度到内置组件的 paint() 方法。
///
/// 支持的组件类型：`Button`、`Label`、`TextField`、`CheckBox`、`Switch`、
/// `Slider`、`ProgressBar`、`Container`、`Row`、`Column`、`Padding`、
/// `Center`、`Expanded`、`SizedBox`、`Card`、`Divider`、`Image`、
/// `ScrollView`、`Stack`。
///
/// 未知组件类型返回空 Vec。
#[must_use]
pub fn default_paint_fn() -> PaintFn {
    Box::new(
        |widget_type: &str, _id: WidgetId, bounds: Rect| -> Vec<PaintOp> {
            match widget_type {
                "Button" => {
                    use rgui_components::button::{Button, ButtonState};
                    let label = "Button";
                    let state = ButtonState::new(label);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Button, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Label" => {
                    use rgui_components::label::{Label, LabelState};
                    let state = LabelState {
                        text: String::new(),
                    };
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Label, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "TextField" => {
                    use rgui_components::text_field::{TextField, TextFieldState};
                    let state = TextFieldState::new("");
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&TextField, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "CheckBox" => {
                    use rgui_components::check_box::{CheckBox, CheckBoxState};
                    let state = CheckBoxState::new("CheckBox");
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&CheckBox, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Switch" => {
                    use rgui_components::switch::{Switch, SwitchState};
                    let state = SwitchState::default();
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Switch, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Slider" => {
                    use rgui_components::slider::{Slider, SliderState};
                    let state = SliderState::new(50.0, 0.0, 100.0);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&Slider, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "ProgressBar" => {
                    use rgui_components::progress_bar::{ProgressBar, ProgressBarState};
                    let state = ProgressBarState::new(0.5);
                    let mut ctx = rgui_core::context::PaintContext::new(bounds);
                    rgui_core::traits::WidgetSpec::paint(&ProgressBar, &state, bounds, &mut ctx);
                    ctx.into_operations()
                },
                "Divider" => {
                    use rgui_components::divider::{Divider, DividerState};
                    let state = DividerState {
                        direction: rgui_core::geometry::Axis::Horizontal,
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
