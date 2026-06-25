//! {{project-name}} — rgui 桌面应用
//!
//! 使用 `cargo generate rgui` 创建。

use rgui::app::{App, AppConfig};
use rgui::{Rect, WidgetId};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建应用配置
    let config = AppConfig::new()
        .title("{{project-name}}")
        .window_size(800.0, 600.0);
    let mut app = App::new(config);

    // 2. 注册交互区域（采用 Qt Signal/Slot 风格的显式连接模型）
    let count = Arc::new(Mutex::new(0i32));
    let c1 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(1),
        Rect::new(350.0, 280.0, 100.0, 40.0),
        "+1",
        move |_| {
            let mut guard = c1.lock().unwrap_or_else(|e| e.into_inner());
            *guard += 1;
            log::info!(target: "rgui::core", "计数: {guard}");
        },
    );

    let c2 = Arc::clone(&count);
    app.register_interaction(
        WidgetId::from_u64(2),
        Rect::new(350.0, 340.0, 100.0, 40.0),
        "reset",
        move |_| {
            *c2.lock().unwrap_or_else(|e| e.into_inner()) = 0;
            log::info!(target: "rgui::core", "重置完成");
        },
    );

    // 3. 运行应用（打开窗口，进入事件循环）
    log::info!(target: "rgui::core", "\n{{project-name}} 已启动。");
    log::info!(target: "rgui::core", "点击窗口中的 [+1] 区域递增计数，[reset] 清空。");
    app.run()
}
