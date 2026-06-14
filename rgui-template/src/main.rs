//! {{project-name}} — rgui 桌面应用
//!
//! 使用 `cargo generate rgui` 创建。

use rgui::app::{App, AppConfig};
use rgui::prelude::*;
use rgui::{Label, LabelState, Rect, WidgetId, WidgetSpec};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建应用配置
    let config = AppConfig::new()
        .title("{{project-name}}")
        .window_size(800.0, 600.0);
    let mut app = App::new(config);
    app.register_defaults();

    // 2. 创建共享状态
    let count = Arc::new(Mutex::new(0i32));
    let label_state = Arc::new(Mutex::new(LabelState {
        text: "点击按钮开始".into(),
    }));

    // 3. 注册交互区域
    let l1 = Arc::clone(&label_state);
    let c1 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(1),
        Rect::new(350.0, 280.0, 100.0, 40.0),
        "+1",
        move |_| {
            let mut guard = c1.lock().unwrap_or_else(|e| e.into_inner());
            *guard += 1;
            l1.lock().unwrap_or_else(|e| e.into_inner()).text =
                format!("计数: {guard}");
        },
    );

    let l2 = Arc::clone(&label_state);
    let c2 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(2),
        Rect::new(350.0, 340.0, 100.0, 40.0),
        "reset",
        move |_| {
            *c2.lock().unwrap_or_else(|e| e.into_inner()) = 0;
            l2.lock().unwrap_or_else(|e| e.into_inner()).text = "重置完成".into();
        },
    );

    // 4. 运行应用（打开窗口，进入事件循环）
    println!("\n{{project-name}} 已启动。");
    println!("点击窗口中的 [+1] 区域递增计数，[重置] 清空。");
    app.run()
}
