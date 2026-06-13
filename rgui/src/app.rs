//! App 启动器——winit 窗口 + 事件循环 + wgpu 渲染 + 交互。

use crate::render::RenderContext;
use rgui_core::geometry::{Point, Rect, Size};
use rgui_core::id::{WidgetId, WindowId};
use rgui_core::registry::WidgetRegistry;
use rgui_platform::event::{Event, Modifiers, MouseButton};
use rgui_platform::hit_test::HitTester;
use rgui_platform::focus::FocusManager;
use std::collections::HashMap as FxHashMap;
use std::fmt;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::WindowAttributes;

/// 交互回调类型。
pub type InteractionCallback = Box<dyn FnMut(&str) + Send>;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub title: String,
    pub window_size: Size,
    pub resizable: bool,
    pub min_size: Option<Size>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "rgui App".to_string(),
            window_size: Size::new(800.0, 600.0),
            resizable: true,
            min_size: Some(Size::new(320.0, 240.0)),
        }
    }
}

impl AppConfig {
    #[must_use] pub fn new() -> Self { Self::default() }
    #[must_use] pub fn title(mut self, t: impl Into<String>) -> Self { self.title = t.into(); self }
    #[must_use] pub fn window_size(mut self, w: f64, h: f64) -> Self { self.window_size = Size::new(w, h); self }
}

#[allow(dead_code)]
pub struct App {
    config: AppConfig,
    registry: WidgetRegistry,
    window_id: WindowId,
    events: Vec<Event>,
    hit_tester: HitTester,
    focus: FocusManager,
    /// 交互区域：widget_id → (Rect, 消息名, 回调)
    interactions: FxHashMap<WidgetId, (Rect, String, InteractionCallback)>,
}

impl App {
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        Self {
            config, registry: WidgetRegistry::new(), window_id: WindowId::new(),
            events: Vec::new(), hit_tester: HitTester::new(), focus: FocusManager::new(),
            interactions: FxHashMap::new(),
        }
    }

    #[must_use] pub fn config(&self) -> &AppConfig { &self.config }
    #[must_use] pub fn registry(&self) -> &WidgetRegistry { &self.registry }
    pub fn registry_mut(&mut self) -> &mut WidgetRegistry { &mut self.registry }
    #[must_use] pub fn window_id(&self) -> WindowId { self.window_id }
    pub fn register_defaults(&mut self) {
        self.registry.register("Button").ok();
        self.registry.register("Label").ok();
        self.registry.register("TextField").ok();
    }
    #[must_use] pub fn events(&self) -> &[Event] { &self.events }

    /// 注册可交互区域。
    ///
    /// - `id`: widget ID
    /// - `bounds`: 在窗口中的边界矩形
    /// - `action`: 触发时传递给回调的事件名
    /// - `cb`: 交互回调
    pub fn register_interaction(
        &mut self, id: WidgetId, bounds: Rect,
        action: impl Into<String>, cb: impl FnMut(&str) + Send + 'static,
    ) {
        self.hit_tester.register(id, bounds);
        self.interactions.insert(id, (bounds, action.into(), Box::new(cb)));
    }

    /// 运行应用。
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut AppHandler::new(self))?;
        Ok(())
    }
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App").field("title", &self.config.title).finish()
    }
}

// ============================================================================
// AppHandler
// ============================================================================

struct AppHandler {
    app: App,
    window: Option<Arc<winit::window::Window>>,
    render_ctx: Option<RenderContext>,
    frame_count: u64,
    mouse_pos: Point,
}

impl AppHandler {
    fn new(app: App) -> Self {
        Self { app, window: None, render_ctx: None, frame_count: 0, mouse_pos: Point::ZERO }
    }

    fn handle_click(&mut self, position: Point) {
        if let Some(hit_id) = self.app.hit_tester.hit_test(position) {
            if let Some((_bounds, action, cb)) = self.app.interactions.get_mut(&hit_id) {
                println!("点击: widget {hit_id:?} at ({}, {}), action: {action}",
                    position.x as i32, position.y as i32);
                cb(action);
                // 记录事件
                self.app.events.push(Event::MouseDown {
                    position,
                    button: MouseButton::Left,
                    modifiers: Modifiers::new(),
                });
            }
        }
    }

    fn convert_event(&self, event: &WindowEvent) -> Option<Event> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                Some(Event::MouseMove {
                    position: Point::new(position.x, position.y),
                    delta: Point::new(0.0, 0.0),
                    modifiers: Modifiers::new(),
                })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    WinitMouseButton::Left => MouseButton::Left,
                    WinitMouseButton::Right => MouseButton::Right,
                    WinitMouseButton::Middle => MouseButton::Middle,
                    WinitMouseButton::Back => MouseButton::Back,
                    WinitMouseButton::Forward => MouseButton::Forward,
                    WinitMouseButton::Other(n) => MouseButton::Other((*n) as u8),
                };
                Some(match state {
                    ElementState::Pressed => Event::MouseDown { position: Point::ZERO, button: btn, modifiers: Modifiers::new() },
                    ElementState::Released => Event::MouseUp { position: Point::ZERO, button: btn, modifiers: Modifiers::new() },
                })
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = match &event.logical_key {
                    WinitKey::Named(n) => convert_named_key(n),
                    WinitKey::Character(c) => convert_char_key(c),
                    _ => return None,
                };
                Some(match event.state {
                    ElementState::Pressed => Event::KeyDown { key, modifiers: Modifiers::new(), repeat: event.repeat },
                    ElementState::Released => Event::KeyUp { key, modifiers: Modifiers::new() },
                })
            }
            WindowEvent::CloseRequested => Some(Event::CloseRequested),
            WindowEvent::Resized(size) => {
                Some(Event::WindowResized { width: size.width as f64, height: size.height as f64 })
            }
            WindowEvent::Focused(f) => {
                if *f { Some(Event::WindowFocused) } else { Some(Event::WindowUnfocused) }
            }
            _ => None,
        }
    }
}

fn convert_named_key(key: &NamedKey) -> rgui_platform::event::Key {
    use rgui_platform::event::Key as Rk;
    match key {
        NamedKey::Enter => Rk::Enter, NamedKey::Tab => Rk::Tab, NamedKey::Space => Rk::Space,
        NamedKey::Backspace => Rk::Backspace, NamedKey::Escape => Rk::Escape,
        NamedKey::Delete => Rk::Delete,
        NamedKey::ArrowLeft => Rk::ArrowLeft, NamedKey::ArrowRight => Rk::ArrowRight,
        NamedKey::ArrowUp => Rk::ArrowUp, NamedKey::ArrowDown => Rk::ArrowDown,
        NamedKey::Home => Rk::Home, NamedKey::End => Rk::End,
        NamedKey::PageUp => Rk::PageUp, NamedKey::PageDown => Rk::PageDown,
        NamedKey::Shift => Rk::Shift, NamedKey::Control => Rk::Ctrl,
        NamedKey::Alt => Rk::Alt, NamedKey::Super => Rk::Meta,
        NamedKey::F1 => Rk::F1, NamedKey::F2 => Rk::F2, NamedKey::F3 => Rk::F3,
        NamedKey::F4 => Rk::F4, NamedKey::F5 => Rk::F5, NamedKey::F6 => Rk::F6,
        NamedKey::F7 => Rk::F7, NamedKey::F8 => Rk::F8, NamedKey::F9 => Rk::F9,
        NamedKey::F10 => Rk::F10, NamedKey::F11 => Rk::F11, NamedKey::F12 => Rk::F12,
        _ => Rk::Enter,
    }
}

fn convert_char_key(c: &str) -> rgui_platform::event::Key {
    use rgui_platform::event::Key as Rk;
    match c.to_uppercase().as_str() {
        "A" => Rk::A, "B" => Rk::B, "C" => Rk::C, "D" => Rk::D, "E" => Rk::E,
        "F" => Rk::F, "G" => Rk::G, "H" => Rk::H, "I" => Rk::I, "J" => Rk::J,
        "K" => Rk::K, "L" => Rk::L, "M" => Rk::M, "N" => Rk::N, "O" => Rk::O,
        "P" => Rk::P, "Q" => Rk::Q, "R" => Rk::R, "S" => Rk::S, "T" => Rk::T,
        "U" => Rk::U, "V" => Rk::V, "W" => Rk::W, "X" => Rk::X, "Y" => Rk::Y, "Z" => Rk::Z,
        "0" => Rk::Digit0, "1" => Rk::Digit1, "2" => Rk::Digit2, "3" => Rk::Digit3,
        "4" => Rk::Digit4, "5" => Rk::Digit5, "6" => Rk::Digit6, "7" => Rk::Digit7,
        "8" => Rk::Digit8, "9" => Rk::Digit9,
        _ => Rk::Enter,
    }
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attr = WindowAttributes::default()
                .with_title(&self.app.config.title)
                .with_inner_size(LogicalSize::new(
                    self.app.config.window_size.width,
                    self.app.config.window_size.height,
                ))
                .with_resizable(self.app.config.resizable);

            let mut attr = attr;
            if let Some(ref s) = self.app.config.min_size {
                attr = attr.with_min_inner_size(LogicalSize::new(s.width, s.height));
            }

            let window = Arc::new(event_loop.create_window(attr).unwrap());
            self.window = Some(Arc::clone(&window));

            match RenderContext::new(Arc::clone(&window)) {
                Ok(ctx) => {
                    println!("GPU: {:?}", ctx);
                    self.render_ctx = Some(ctx);
                }
                Err(e) => eprintln!("渲染初始化失败: {e}"),
            }

            println!("rgui 窗口已创建: {}", self.app.config.title);
            println!("点击窗口中的按钮区域触发交互...");
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Point::new(position.x, position.y);
            }

            WindowEvent::MouseInput { state: ElementState::Pressed, button: WinitMouseButton::Left, .. } => {
                self.handle_click(self.mouse_pos);
            }

            WindowEvent::Resized(size) => {
                if let Some(ref mut ctx) = self.render_ctx {
                    ctx.resize(size.width, size.height);
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(ref mut ctx) = self.render_ctx {
                    match ctx.render() {
                        Ok(()) => self.frame_count += 1,
                        Err(e) => eprintln!("渲染: {e}"),
                    }
                }
            }
            _ => {
                if let Some(rgui_event) = self.convert_event(&event) {
                    self.app.events.push(rgui_event);
                }
            }
        }
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        println!("rgui 已退出（{} 帧）", self.frame_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creation() {
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut app = App::new(config);
        assert_eq!(app.config().title, "Test");
        app.register_defaults();
    }

    #[test]
    fn app_events_start_empty() {
        let app = App::new(AppConfig::default());
        assert!(app.events().is_empty());
    }

    #[test]
    fn app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.title, "rgui App");
    }
}
