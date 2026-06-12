//! 基线测量：Rust 快速重启端到端延迟
//!
//! 直接测量：
//!   项 2: 进程启动 + 窗口创建
//!   项 5: 首帧事件延迟
//!
//! 项 1（增量编译）通过脚本测量。
//! 项 3（状态反序列化）依赖 V7。
//! 项 4（组件树重建）依赖阶段 1 框架雏形。

use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

fn main() {
    let t0 = Instant::now();

    let event_loop = EventLoop::new().expect("创建 EventLoop 失败");
    let t_eloop = t0.elapsed();

    let mut app = MeasureApp {
        t0,
        t_eloop,
        t_window: None,
        t_first_event: None,
        started: false,
    };

    event_loop.run_app(&mut app).expect("事件循环退出");
}

struct MeasureApp {
    t0: Instant,
    t_eloop: std::time::Duration,
    t_window: Option<std::time::Duration>,
    t_first_event: Option<std::time::Duration>,
    started: bool,
}

impl ApplicationHandler for MeasureApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.started {
            return;
        }
        self.started = true;

        let window = event_loop
            .create_window(Window::default_attributes())
            .expect("创建窗口失败");
        self.t_window = Some(self.t0.elapsed());

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.t_first_event.is_none() {
            self.t_first_event = Some(self.t0.elapsed());
            println!(
                "event_loop_us={} window_us={} first_event_us={}",
                self.t_eloop.as_micros(),
                self.t_window.unwrap().as_micros(),
                self.t_first_event.unwrap().as_micros(),
            );
            // 拿到数据后立即退出，无须渲染
            event_loop.exit();
        }

        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
        }
    }
}
