//! rgui 计数器——使用 `html!` 声明式语法。
//!
//! 本示例演示 `html!` 宏构建 WidgetView 树，
//! 并通过 `view_to_paint_layers` 桥接到渲染管线。

use rgui::app::{App, AppConfig};
use rgui::{
    AppMessage, Button, ButtonState, Color, Label, LabelState, PaintContext, PaintLayerData,
    PropValue, Rect, WidgetId, WidgetSpec, WidgetView, html,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// 消息类型
// ============================================================================

#[derive(Debug, Clone, PartialEq, AppMessage)]
enum CounterMsg {
    Increment,
    Reset,
}

// ============================================================================
// WidgetView → PaintLayerData 桥接
// ============================================================================

fn view_to_paint_layers<M: AppMessage>(
    view: &WidgetView<M>,
    parent_bounds: Rect,
) -> Vec<PaintLayerData> {
    let mut layers = Vec::new();
    let mut z = 0i32;
    walk_view(view, parent_bounds, &mut layers, &mut z);
    layers
}

fn get_prop_str(props: &BTreeMap<&'static str, PropValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|v| match v {
            PropValue::Str(s) => Some(s.as_ref().to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

fn walk_view<M: AppMessage>(
    view: &WidgetView<M>,
    bounds: Rect,
    layers: &mut Vec<PaintLayerData>,
    z: &mut i32,
) {
    let widget_id = view.id.unwrap_or_default();
    let mut ctx = PaintContext::new(bounds);

    match view.widget_type {
        "Button" => {
            let label = get_prop_str(&view.props, "label");
            Button.paint(&ButtonState::new(label), bounds, &mut ctx);
        },
        "Label" => {
            let text = get_prop_str(&view.props, "text");
            Label.paint(&LabelState { text }, bounds, &mut ctx);
        },
        "Column" | "Row" | "Container" | "Padding" | "Center" | "SizedBox" | "Expanded"
        | "Card" | "Stack" => {},
        _ => {},
    }

    let ops = ctx.into_operations();
    if !ops.is_empty() {
        layers.push(PaintLayerData::new(widget_id, *z, bounds, ops));
        *z += 1;
    }

    for child in &view.children {
        walk_view(child, bounds, layers, z);
    }
}

// ============================================================================
// 主函数
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui — 计数器（html! 语法）")
            .window_size(400.0, 300.0),
    );
    app.register_defaults();

    let count = Arc::new(Mutex::new(0_i32));

    // [+1] 按钮交互
    let c1 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(1),
        Rect::new(50.0, 80.0, 100.0, 40.0),
        "+1",
        move |_| {
            *c1.lock().unwrap() += 1;
        },
    );

    // [重置] 按钮交互
    let c2 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(2),
        Rect::new(170.0, 80.0, 100.0, 40.0),
        "reset",
        move |_| {
            *c2.lock().unwrap() = 0;
        },
    );

    // 场景构建回调——使用 html! 声明式 UI，桥接到渲染管线
    let count_for_view = Arc::clone(&count);
    app.set_scene_builder(move |_frame: u64, width: u32, height: u32| {
        let w = width as f64;
        let h = height as f64;
        let cnt = *count_for_view.lock().unwrap();

        // html! 声明式 UI 定义
        let view: WidgetView<CounterMsg> = html! {
            <Column>
                <Label text="rgui 计数器演示" />
                <Row gap="8.0">
                    <Button id="1" label="+1" on:click={CounterMsg::Increment} />
                    <Button id="2" label="重置" on:click={CounterMsg::Reset} />
                </Row>
                <Label text={format!("计数: {}", cnt)} />
            </Column>
        };

        // 背景层
        let mut layers: Vec<PaintLayerData> = Vec::new();
        let mut bg_ctx = PaintContext::new(Rect::new(0.0, 0.0, w, h));
        bg_ctx.fill_rect(
            Rect::new(0.0, 0.0, w, h),
            Color::new(14.0 / 255.0, 18.0 / 255.0, 28.0 / 255.0, 1.0),
            0.0,
        );
        layers.push(PaintLayerData::new(
            WidgetId::from_u64(0),
            -1,
            Rect::new(0.0, 0.0, w, h),
            bg_ctx.into_operations(),
        ));

        // 从 WidgetView 树生成组件绘制层
        let view_layers = view_to_paint_layers(&view, Rect::new(0.0, 0.0, w, h));
        layers.extend(view_layers);

        layers
    });

    println!("=== rgui 计数器（html! 声明式语法）===\n");
    println!("UI 由 html! 宏声明，通过 view_to_paint_layers 桥接到渲染管线。");
    println!();
    println!("点击窗口按钮： [+1]  [重置]");

    app.run()
}
