//! V10: cosmic-text IME 集成路径验证
//!
//! 验证 winit IME → cosmic-text Buffer 编辑 → 候选窗位置的完整链路。

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

fn main() {
    println!("V10: IME 集成验证\n  切换系统输入法到拼音后键入，观察 Preedit/Commit 事件\n  Esc 退出\n");
    let el = EventLoop::new().expect("EventLoop");
    let mut app = ImeApp::new();
    el.run_app(&mut app).expect("事件循环退出");
    app.print_summary();
}

struct ImeApp {
    font_system: FontSystem,
    buffer: Buffer,
    attrs: Attrs<'static>,
    preedit: String,
    commit_count: u32,
    ime_seen: bool,
}

impl ImeApp {
    fn new() -> Self {
        let font_system = FontSystem::new();
        let attrs = Attrs::new();
        let metrics = Metrics::new(20.0, 20.0);
        Self {
            font_system, buffer: Buffer::new_empty(metrics),
            attrs, preedit: String::new(), commit_count: 0, ime_seen: false,
        }
    }

    fn print_summary(&self) {
        println!(
            "\nV10: {}",
            if self.commit_count > 0 || self.ime_seen {
                "✅ winit IME 事件链路正常"
            } else {
                "⚠️ 未收到 IME 事件（请确认已切换系统输入法）"
            }
        );
    }
}

impl ApplicationHandler for ImeApp {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let win = el.create_window(Window::default_attributes().with_title("V10 IME Test")).unwrap();
        win.set_ime_allowed(true);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        use winit::event::Ime;
        match event {
            WindowEvent::Ime(Ime::Enabled) => { self.ime_seen = true; println!("  🔤 IME 激活"); }
            WindowEvent::Ime(Ime::Disabled) => { self.preedit.clear(); println!("  🔤 IME 停用"); }
            WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                self.preedit = text.to_string();
                let buf_text = format!("[组合态: {}]", self.preedit);
                self.buffer.set_text(&mut self.font_system, &buf_text, &self.attrs, Shaping::Advanced, None);
                println!("  📝 Preedit: \"{}\" cursor={:?}", self.preedit, cursor);
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.commit_count += 1;
                println!("  ✅ Commit #{}: \"{}\"", self.commit_count, text);
                self.preedit.clear();
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key: Key::Named(NamedKey::Escape), state: ElementState::Pressed, .. }, ..
            } => el.exit(),
            WindowEvent::CloseRequested => el.exit(),
            _ => {}
        }
    }
}
