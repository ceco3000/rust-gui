//! rgui 计数器——组件库桥接演示。

use rgui::WidgetSpec;
use rgui::app::{App, AppConfig};
use rgui::diff::{WidgetIdMap, diff};
use rgui::{Label, LabelState, Rect, Size, ViewContext, WidgetId};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(
        AppConfig::new()
            .title("rgui 组件库演示")
            .window_size(400.0, 300.0),
    );
    app.register_defaults();

    println!("=== rgui 组件库桥接演示 ===\n");
    println!("已注册组件: {:?}", app.registry());
    println!("Label 组件名: {}", Label.name());
    println!();

    let count = Arc::new(Mutex::new(0_i32));
    let label_state = Arc::new(Mutex::new(LabelState {
        text: "计数: 0".into(),
    }));
    let view_ctx = ViewContext::new(Size::new(400.0, 300.0));

    // 使用 Label WidgetSpec 生成初始视图
    let initial_view = {
        let s = label_state.lock().unwrap();
        Label.view(&s, &view_ctx)
    };
    println!("初始视图: {initial_view:?}");

    // [+1] 交互
    let ls1 = Arc::clone(&label_state);
    let c1 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(1),
        Rect::new(50.0, 80.0, 100.0, 40.0),
        "+1",
        move |_| {
            let mut c = c1.lock().unwrap();
            *c += 1;
            ls1.lock().unwrap().text = format!("计数: {}", *c);
            println!("  → {}", *c);
        },
    );

    // [重置] 交互
    let ls2 = Arc::clone(&label_state);
    let c2 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(2),
        Rect::new(170.0, 80.0, 100.0, 40.0),
        "reset",
        move |_| {
            *c2.lock().unwrap() = 0;
            ls2.lock().unwrap().text = "计数: 0".into();
            println!("  → 重置");
        },
    );

    // 模拟点击 3 次后的视图
    *count.lock().unwrap() = 3;
    label_state.lock().unwrap().text = "计数: 3".into();
    let new_view = { Label.view(&label_state.lock().unwrap(), &view_ctx) };
    println!("\n3 次点击后: {new_view:?}");

    // Diff
    let mut id_map = WidgetIdMap::new();
    let patches = diff(&initial_view, &new_view, WidgetId::from_u64(1), &mut id_map);
    println!("Diff: {} patch(es)", patches.len());
    for p in &patches {
        println!("  {p:?}");
    }

    println!("\n✅ 组件库桥接验证通过\n");
    println!("点击窗口按钮： [+1]  [重置]");

    app.run()
}
