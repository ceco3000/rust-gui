//! App 启动器——winit 窗口 + 事件循环 + wgpu 渲染 + 交互。

use rgui_core::geometry::{Point, Rect, Size};
use rgui_core::id::{WidgetId, WindowId};
use rgui_core::registry::WidgetRegistry;
use rgui_platform::event::{Event, Modifiers, MouseButton};
use rgui_platform::focus::FocusManager;
use rgui_platform::hit_test::HitTester;
use rgui_render::{
    PaintLayerData, RenderBackend, RenderParams, SceneGraph, TextRenderer, VelloBackend,
    build_scene_from_paint_data,
};
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

/// 场景构建回调类型。
///
/// 回调接收帧计数、窗口宽度、窗口高度（像素），返回 `PaintLayerData` 列表。
/// 框架自动调用 `build_scene_from_paint_data` 将其转换为 SceneGraph，
/// 并传入 `TextRenderer` 以启用真正的字形渲染。
/// 在每帧 `RedrawRequested` 时调用。如果未设置，渲染循环使用空场景图。
pub type SceneBuilder = Box<dyn FnMut(u64, u32, u32) -> Vec<PaintLayerData> + Send>;

#[allow(clippy::type_complexity)]
/// tick() 布局回调。
pub type LayoutCallback<'a> = Box<dyn FnOnce(&mut App) + 'a>;
#[allow(clippy::type_complexity)]
/// tick() 无障碍回调。
pub type A11yCallback<'a> = Box<dyn FnOnce(&mut App) + 'a>;
#[allow(clippy::type_complexity)]
/// tick() 渲染回调。
pub type RenderCallback<'a> = Box<dyn FnOnce(&mut App, &[Event]) -> Result<(), String> + 'a>;

/// 应用配置。
///
/// 控制窗口标题、初始尺寸、是否可缩放和最小尺寸。
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// 窗口标题。
    pub title: String,
    /// 初始窗口尺寸。
    pub window_size: Size,
    /// 是否可缩放。
    pub resizable: bool,
    /// 最小窗口尺寸。
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
    /// 创建默认配置。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// 设置窗口标题（builder 风格）。
    #[must_use]
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    /// 设置初始窗口尺寸（builder 风格）。
    #[must_use]
    pub fn window_size(mut self, w: f64, h: f64) -> Self {
        self.window_size = Size::new(w, h);
        self
    }
}

/// rgui 应用实例。
///
/// 管理窗口配置、注册表、事件队列、命中测试、焦点和交互区域。
/// 通过 `run()` 方法启动 winit 事件循环。
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
    /// 可选的场景构建回调（每帧调用生成 SceneGraph）。
    scene_builder: Option<SceneBuilder>,
}

impl App {
    /// 创建应用实例。
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            registry: WidgetRegistry::new(),
            window_id: WindowId::new(),
            events: Vec::new(),
            hit_tester: HitTester::new(),
            focus: FocusManager::new(),
            interactions: FxHashMap::new(),
            scene_builder: None,
        }
    }

    /// 返回应用配置。
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
    /// 返回 widget 注册表。
    #[must_use]
    pub fn registry(&self) -> &WidgetRegistry {
        &self.registry
    }
    /// 返回可变 widget 注册表。
    pub fn registry_mut(&mut self) -> &mut WidgetRegistry {
        &mut self.registry
    }
    /// 返回窗口 ID。
    #[must_use]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
    /// 注册内置组件（Button、Label、TextField）。
    pub fn register_defaults(&mut self) {
        self.registry.register("Button").ok();
        self.registry.register("Label").ok();
        self.registry.register("TextField").ok();
    }
    /// 返回当前事件列表。
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// 注册可交互区域。
    ///
    /// - `id`: widget ID
    /// - `bounds`: 在窗口中的边界矩形
    /// - `action`: 触发时传递给回调的事件名
    /// - `cb`: 交互回调
    pub fn register_interaction(
        &mut self,
        id: WidgetId,
        bounds: Rect,
        action: impl Into<String>,
        cb: impl FnMut(&str) + Send + 'static,
    ) {
        self.hit_tester.register(id, bounds);
        self.interactions
            .insert(id, (bounds, action.into(), Box::new(cb)));
    }

    /// 设置场景构建回调。
    ///
    /// 回调在每帧渲染前调用，接收帧计数、窗口宽度和高度（像素），
    /// 返回 `Vec<PaintLayerData>`。框架自动将其转换为 SceneGraph
    /// 并传入 `TextRenderer` 以启用字形渲染。
    pub fn set_scene_builder(
        &mut self,
        builder: impl FnMut(u64, u32, u32) -> Vec<PaintLayerData> + Send + 'static,
    ) {
        self.scene_builder = Some(Box::new(builder));
    }

    /// 运行应用。
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut AppHandler::new(self))?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    /// 框架主循环（每帧调用），D3 §4.1。
    ///
    /// 固定执行顺序：事件分发 → 布局 → 无障碍 → 场景图 → GPU 提交
    pub fn tick(
        &mut self,
        events: Vec<Event>,
        layout_fn: LayoutCallback,
        a11y_fn: A11yCallback,
        render_fn: RenderCallback,
    ) -> Result<(), String> {
        for event in &events {
            self.events.push(event.clone());
        }
        self.hit_tester.clear();
        layout_fn(self);
        a11y_fn(self);
        render_fn(self, &events)?;
        Ok(())
    }

    /// 返回未处理事件数量。
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// 清空事件队列。
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("title", &self.config.title)
            .finish()
    }
}

// ============================================================================
// AppHandler
// ============================================================================

struct AppHandler {
    app: App,
    window: Option<Arc<winit::window::Window>>,
    /// Vello GPU 渲染后端（直接持有，非 facade 封装）。
    /// 符合 D8 R06 架构约束：facade 仅通过 RenderBackend trait 接口委托渲染。
    render_ctx: Option<VelloBackend>,
    /// 文本渲染器（字形塑形 + atlas 管理）。
    /// 在每帧渲染时传递给 scene_builder，启用真正的字形渲染。
    text_renderer: TextRenderer,
    frame_count: u64,
    mouse_pos: Point,
    /// DPI 缩放因子（逻辑像素 → 物理像素比例）。
    /// 从 winit `window.scale_factor()` 读取，默认 1.0。
    scale_factor: f64,
    /// 当前窗口宽度（物理像素），用于构造 RenderParams。
    width: u32,
    /// 当前窗口高度（物理像素），用于构造 RenderParams。
    height: u32,
    /// 场景构建回调（从 App 移入）。
    scene_builder: Option<SceneBuilder>,
}

impl AppHandler {
    fn new(mut app: App) -> Self {
        let scene_builder = app.scene_builder.take();
        Self {
            app,
            window: None,
            render_ctx: None,
            text_renderer: TextRenderer::new(rgui_render::TextureId(0)),
            frame_count: 0,
            mouse_pos: Point::ZERO,
            scale_factor: 1.0,
            width: 0,
            height: 0,
            scene_builder,
        }
    }

    fn handle_click(&mut self, position: Point) {
        if let Some(hit_id) = self.app.hit_tester.hit_test(position) {
            if let Some((_bounds, action, cb)) = self.app.interactions.get_mut(&hit_id) {
                println!(
                    "点击: widget {hit_id:?} at ({}, {}), action: {action}",
                    position.x as i32, position.y as i32
                );
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
            WindowEvent::CursorMoved { position, .. } => Some(Event::MouseMove {
                position: Point::new(
                    position.x / self.scale_factor,
                    position.y / self.scale_factor,
                ),
                delta: Point::new(0.0, 0.0),
                modifiers: Modifiers::new(),
            }),
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
                    ElementState::Pressed => Event::MouseDown {
                        position: self.mouse_pos,
                        button: btn,
                        modifiers: Modifiers::new(),
                    },
                    ElementState::Released => Event::MouseUp {
                        position: self.mouse_pos,
                        button: btn,
                        modifiers: Modifiers::new(),
                    },
                })
            },
            WindowEvent::KeyboardInput { event, .. } => {
                let key = match &event.logical_key {
                    WinitKey::Named(n) => convert_named_key(n),
                    WinitKey::Character(c) => convert_char_key(c),
                    _ => return None,
                };
                Some(match event.state {
                    ElementState::Pressed => Event::KeyDown {
                        key,
                        modifiers: Modifiers::new(),
                        repeat: event.repeat,
                    },
                    ElementState::Released => Event::KeyUp {
                        key,
                        modifiers: Modifiers::new(),
                    },
                })
            },
            WindowEvent::CloseRequested => Some(Event::CloseRequested),
            WindowEvent::Resized(size) => Some(Event::WindowResized {
                width: size.width as f64,
                height: size.height as f64,
            }),
            WindowEvent::Focused(f) => {
                if *f {
                    Some(Event::WindowFocused)
                } else {
                    Some(Event::WindowUnfocused)
                }
            },
            _ => None,
        }
    }
}

fn convert_named_key(key: &NamedKey) -> rgui_platform::event::Key {
    use rgui_platform::event::Key as Rk;
    match key {
        NamedKey::Enter => Rk::Enter,
        NamedKey::Tab => Rk::Tab,
        NamedKey::Space => Rk::Space,
        NamedKey::Backspace => Rk::Backspace,
        NamedKey::Escape => Rk::Escape,
        NamedKey::Delete => Rk::Delete,
        NamedKey::ArrowLeft => Rk::ArrowLeft,
        NamedKey::ArrowRight => Rk::ArrowRight,
        NamedKey::ArrowUp => Rk::ArrowUp,
        NamedKey::ArrowDown => Rk::ArrowDown,
        NamedKey::Home => Rk::Home,
        NamedKey::End => Rk::End,
        NamedKey::PageUp => Rk::PageUp,
        NamedKey::PageDown => Rk::PageDown,
        NamedKey::Shift => Rk::Shift,
        NamedKey::Control => Rk::Ctrl,
        NamedKey::Alt => Rk::Alt,
        NamedKey::Super => Rk::Meta,
        NamedKey::F1 => Rk::F1,
        NamedKey::F2 => Rk::F2,
        NamedKey::F3 => Rk::F3,
        NamedKey::F4 => Rk::F4,
        NamedKey::F5 => Rk::F5,
        NamedKey::F6 => Rk::F6,
        NamedKey::F7 => Rk::F7,
        NamedKey::F8 => Rk::F8,
        NamedKey::F9 => Rk::F9,
        NamedKey::F10 => Rk::F10,
        NamedKey::F11 => Rk::F11,
        NamedKey::F12 => Rk::F12,
        _ => Rk::Enter,
    }
}

fn convert_char_key(c: &str) -> rgui_platform::event::Key {
    use rgui_platform::event::Key as Rk;
    match c.to_uppercase().as_str() {
        "A" => Rk::A,
        "B" => Rk::B,
        "C" => Rk::C,
        "D" => Rk::D,
        "E" => Rk::E,
        "F" => Rk::F,
        "G" => Rk::G,
        "H" => Rk::H,
        "I" => Rk::I,
        "J" => Rk::J,
        "K" => Rk::K,
        "L" => Rk::L,
        "M" => Rk::M,
        "N" => Rk::N,
        "O" => Rk::O,
        "P" => Rk::P,
        "Q" => Rk::Q,
        "R" => Rk::R,
        "S" => Rk::S,
        "T" => Rk::T,
        "U" => Rk::U,
        "V" => Rk::V,
        "W" => Rk::W,
        "X" => Rk::X,
        "Y" => Rk::Y,
        "Z" => Rk::Z,
        "0" => Rk::Digit0,
        "1" => Rk::Digit1,
        "2" => Rk::Digit2,
        "3" => Rk::Digit3,
        "4" => Rk::Digit4,
        "5" => Rk::Digit5,
        "6" => Rk::Digit6,
        "7" => Rk::Digit7,
        "8" => Rk::Digit8,
        "9" => Rk::Digit9,
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
            let size = window.inner_size();
            let w = size.width;
            let h = size.height;
            self.scale_factor = window.scale_factor();
            self.window = Some(Arc::clone(&window));

            match VelloBackend::new(Arc::clone(&window), w, h) {
                Ok(ctx) => {
                    println!("GPU: Vello (GPU) {w}x{h} (scale: {:.2})", self.scale_factor);
                    self.render_ctx = Some(ctx);
                    self.width = w;
                    self.height = h;
                },
                Err(e) => eprintln!("渲染初始化失败: {e}"),
            }

            println!("rgui 窗口已创建: {}", self.app.config.title);
            println!("点击窗口中的按钮区域触发交互...");
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        mut event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Point::new(
                    position.x / self.scale_factor,
                    position.y / self.scale_factor,
                );
            },

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: WinitMouseButton::Left,
                ..
            } => {
                self.handle_click(self.mouse_pos);
            },

            WindowEvent::Resized(size) => {
                self.width = size.width;
                self.height = size.height;
            },

            WindowEvent::RedrawRequested => {
                if let Some(ref mut ctx) = self.render_ctx {
                    let frame = self.frame_count;
                    // 使用逻辑像素尺寸供组件 paint/measure，物理像素供 RenderBackend。
                    let logical_w = (self.width as f64 / self.scale_factor).max(1.0);
                    let logical_h = (self.height as f64 / self.scale_factor).max(1.0);
                    let scene = if let Some(ref mut builder) = self.scene_builder {
                        let layers = builder(frame, logical_w as u32, logical_h as u32);
                        build_scene_from_paint_data(&layers, frame, Some(&self.text_renderer))
                    } else {
                        SceneGraph::new(frame)
                    };
                    let params = RenderParams {
                        width: self.width,
                        height: self.height,
                        scale_factor: self.scale_factor,
                        clear_color: Some(rgui_core::Color::new(
                            14.0 / 255.0,
                            18.0 / 255.0,
                            28.0 / 255.0,
                            1.0,
                        )),
                        ..Default::default()
                    };
                    // 将字形 Atlas 像素数据传入 Vello 后端
                    if self.text_renderer.is_dirty() {
                        let (aw, ah) = self.text_renderer.atlas_dimensions();
                        let pixels = self.text_renderer.atlas_pixels();
                        ctx.set_atlas_data(aw, ah, &pixels);
                        self.text_renderer.clear_dirty();
                    }
                    match ctx.render(&scene, &params) {
                        Ok(()) => self.frame_count += 1,
                        Err(e) => eprintln!("渲染: {e}"),
                    }
                }
            },
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                ref mut inner_size_writer,
            } => {
                self.scale_factor = scale_factor;
                // 更新物理尺寸，winit 在 DPI 变化后返回新的物理尺寸
                if let Some(window) = &self.window {
                    let new_size = window.inner_size();
                    self.width = new_size.width;
                    self.height = new_size.height;
                }
                // 通知 Vello 后端 resize
                if let Some(ref mut ctx) = self.render_ctx {
                    ctx.resize(self.width, self.height);
                }
                // 回应 winit：告知我们已接受此尺寸
                let _ = inner_size_writer
                    .request_inner_size(winit::dpi::PhysicalSize::new(self.width, self.height));
                // 转发事件给组件层
                self.app
                    .events
                    .push(Event::ScaleFactorChanged { scale_factor });
            },
            _ => {
                if let Some(rgui_event) = self.convert_event(&event) {
                    self.app.events.push(rgui_event);
                }
            },
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
    fn tick_dispatches_events() {
        let mut app = App::new(AppConfig::default());
        app.tick(
            vec![Event::WindowFocused, Event::CloseRequested],
            Box::new(|_| {}),
            Box::new(|_| {}),
            Box::new(|_, _| Ok(())),
        )
        .unwrap();
        assert_eq!(app.event_count(), 2);
    }

    #[test]
    fn tick_reports_render_error() {
        let mut app = App::new(AppConfig::default());
        let result = app.tick(
            vec![],
            Box::new(|_| {}),
            Box::new(|_| {}),
            Box::new(|_, _| Err("GPU 错误".into())),
        );
        assert!(result.is_err());
    }

    #[test]
    fn tick_calls_layout_and_a11y() {
        use std::cell::Cell;
        let mut app = App::new(AppConfig::default());
        let layout_called = Cell::new(false);
        let a11y_called = Cell::new(false);
        app.tick(
            vec![],
            Box::new(|_| layout_called.set(true)),
            Box::new(|_| a11y_called.set(true)),
            Box::new(|_, _| Ok(())),
        )
        .unwrap();
        assert!(layout_called.get());
        assert!(a11y_called.get());
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
