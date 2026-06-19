//! App 启动器——winit 窗口 + 事件循环 + wgpu 渲染 + 交互。

use rgui_core::context::UpdateContext;
use rgui_core::geometry::{Point, Rect, Size};
use rgui_core::id::{WidgetId, WindowId};
use rgui_core::registry::WidgetRegistry;
#[cfg(feature = "devtools")]
use rgui_core::traits::AppMessage;
use rgui_core::traits::EventResult;
use rgui_platform::event::{Event, Modifiers, MouseButton};
use rgui_platform::focus::FocusManager;
use rgui_platform::hit_test::HitTester;
use rgui_render::{
    RenderBackend, RenderBackendFactory, RenderParams, SceneGraph, TextRenderer, VelloBackend,
};
use std::collections::HashMap as FxHashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::WindowAttributes;

/// 交互回调类型。
pub type InteractionCallback = Box<dyn FnMut(&str) + Send>;

/// Widget 实例更新处理器（类型擦除）。
///
/// 接收动作名称和 UpdateContext，返回 EventResult 控制事件传播。
pub type WidgetUpdateHandler =
    Box<dyn FnMut(&str, &mut UpdateContext) -> EventResult<String> + Send>;

/// 视图场景构建回调类型。
///
/// 回调接收帧计数、窗口宽度、窗口高度（逻辑像素）和文本渲染器引用，
/// 直接返回 `SceneGraph`。用于 `html!` 宏 + `build_scene_from_view` 的声明式路径。
pub type ViewSceneBuilder = Box<dyn FnMut(u64, u32, u32, &TextRenderer) -> SceneGraph + Send>;

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
    /// 可选 `.rgui` 文件路径，作为视图源（替代 `set_view_scene_builder`）。
    /// `App::load_rgui::<M>()` 从此字段读取路径。
    pub rgui_path: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "rgui App".to_string(),
            window_size: Size::new(800.0, 600.0),
            resizable: true,
            min_size: Some(Size::new(320.0, 240.0)),
            rgui_path: None,
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
    /// 设置 .rgui 文件路径（builder 风格）。
    #[must_use]
    pub fn rgui_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.rgui_path = Some(path.into());
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
    /// Widget 实例更新处理器（WidgetSpec 路径）。
    /// 如果存在，handle_click 优先通过此处理器调用 update()，再根据 EventResult 决定是否回调旧路径。
    widget_instances: FxHashMap<WidgetId, WidgetUpdateHandler>,
    /// 可选的视图场景构建回调（直接返回 SceneGraph）。
    view_scene_builder: Option<ViewSceneBuilder>,
    /// 当前 DPI 缩放因子（逻辑像素 → 物理像素比例）。
    /// 从 winit `window.scale_factor()` 读取，默认 1.0。
    pub(crate) scale_factor: f64,
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
            widget_instances: FxHashMap::new(),
            view_scene_builder: None,
            scale_factor: 1.0,
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
    /// 返回当前 DPI 缩放因子。
    ///
    /// 逻辑像素 × `scale_factor` = 物理像素。
    /// 在普通 1× 显示器上为 1.0，Mac Retina 为 2.0，Windows 150% 缩放为 1.5。
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
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

    /// 注册 WidgetSpec 实例的更新处理器。
    ///
    /// - `id`: widget ID（需与 register_interaction 的 id 一致）
    /// - `handler`: 事件处理器，接收动作名称，返回 EventResult
    ///
    /// 当 handle_click 命中此 widget 时，优先调用 handler，
    /// 根据返回的 EventResult 决定是否继续传播到旧 register_interaction 回调。
    pub fn register_widget_instance(
        &mut self,
        id: WidgetId,
        handler: impl FnMut(&str, &mut UpdateContext) -> EventResult<String> + Send + 'static,
    ) {
        self.widget_instances.insert(id, Box::new(handler));
    }

    /// 设置视图场景构建回调（html! 声明式路径）。
    ///
    /// 回调在每帧渲染前调用，接收帧计数、窗口宽度和高度（像素），
    /// 直接返回 `SceneGraph`。
    ///
    /// 与 `build_scene_from_view` 配合使用，实现 WidgetView → SceneGraph 的端到端管线。
    pub fn set_view_scene_builder(
        &mut self,
        builder: impl FnMut(u64, u32, u32, &TextRenderer) -> SceneGraph + Send + 'static,
    ) {
        self.view_scene_builder = Some(Box::new(builder));
    }

    /// 从 `.rgui` 文件加载视图（路径从 `config.rgui_path` 读取）。
    ///
    /// `M` 为 AppMessage 类型，用于解析器泛型参数。
    /// 内部创建 [`RguiHotReload`]，每帧轮询文件变更，
    /// 变更时重新解析 → `compute_view_layout` → `build_scene_from_view` → `SceneGraph`。
    ///
    /// 必须在 `run()` 之前调用。返回 `Err` 如果 config 未设置 `rgui_path`。
    ///
    /// 解析失败时保持旧视图，通过 stderr 报告错误（D7 §9 降级策略）。
    #[cfg(feature = "devtools")]
    pub fn load_rgui<M: AppMessage>(
        &mut self,
    ) -> Result<(), rgui_devtools::rgui_hot_reload::RguiHotReloadError> {
        use rgui_core::geometry::Size;
        use rgui_devtools::config::HotReloadConfig;
        use rgui_devtools::rgui_hot_reload::RguiHotReload;

        let rgui_path = self.config.rgui_path.as_ref().ok_or_else(|| {
            rgui_devtools::rgui_hot_reload::RguiHotReloadError::Watch(
                "AppConfig 未设置 rgui_path".to_string(),
            )
        })?;

        // 构建 HotReloadConfig：监控 .rgui 文件所在目录
        let watch_dir = rgui_path.parent().unwrap_or(std::path::Path::new("."));
        let config = HotReloadConfig::default().with_watch_paths(vec![watch_dir.to_path_buf()]);

        let mut hot_reload = RguiHotReload::<M>::new(&config, rgui_path)?;
        let mut current_view = hot_reload.current_view().clone();

        // 计算初始布局
        let available = Size::new(
            self.config.window_size.width,
            self.config.window_size.height,
        );
        let mut layout_engine = {
            let mut view = current_view.clone();
            rgui_render::compute_view_layout(&mut view, available)
        };

        let builder = move |frame_count: u64,
                            width: u32,
                            height: u32,
                            text_renderer: &TextRenderer|
              -> SceneGraph {
            match hot_reload.check_and_reload() {
                Ok(Some(new_view)) => {
                    let available = Size::new(f64::from(width), f64::from(height));
                    let mut view = new_view.clone();
                    let engine = rgui_render::compute_view_layout(&mut view, available);
                    layout_engine = engine;
                    current_view = new_view;
                },
                Ok(None) => {
                    // 无变更，使用现有视图
                },
                Err(e) => {
                    // 解析失败 → 保持旧视图（D7 §9 降级策略）
                    eprintln!("[rgui] .rgui 热重载失败（保持旧视图）: {e}");
                },
            }

            let paint_fn = crate::paint_factory::default_paint_fn::<M>();
            rgui_render::build_scene_from_view(
                &current_view,
                &layout_engine,
                &paint_fn,
                frame_count,
                Some(text_renderer),
            )
        };

        self.set_view_scene_builder(builder);
        Ok(())
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
    /// 当前活跃的渲染后端（trait object，支持运行时切换）。
    /// 初始为 VelloBackend；渲染失败时在帧边界切换为 SkiaBackend。
    render_ctx: Option<Box<dyn RenderBackend>>,
    /// 待切换标志：当前帧 Vello 渲染失败后设为 true，
    /// 下一帧开始时切换为 Skia。
    backend_fallback_pending: bool,
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
    /// 视图场景构建回调（从 App 移入）。
    view_scene_builder: Option<ViewSceneBuilder>,
}

impl AppHandler {
    fn new(mut app: App) -> Self {
        let view_scene_builder = app.view_scene_builder.take();
        Self {
            app,
            window: None,
            render_ctx: None,
            backend_fallback_pending: false,
            text_renderer: TextRenderer::new(rgui_render::TextureId(0)),
            frame_count: 0,
            mouse_pos: Point::ZERO,
            scale_factor: 1.0,
            width: 0,
            height: 0,
            view_scene_builder,
        }
    }

    fn handle_click(&mut self, position: Point) {
        if let Some(hit_id) = self.app.hit_tester.hit_test(position) {
            // 新路径：如果命中 widget 有 WidgetSpec 实例处理器，优先调用
            let mut update_ctx = UpdateContext::new();
            if let Some(handler) = self.app.widget_instances.get_mut(&hit_id) {
                let action = self
                    .app
                    .interactions
                    .get(&hit_id)
                    .map(|(_, a, _)| a.clone())
                    .unwrap_or_default();
                let result = handler(&action, &mut update_ctx);
                match result {
                    EventResult::Handled => {
                        // 组件消费了事件，停止，不调用旧回调
                        self.app.events.push(Event::MouseDown {
                            position,
                            button: MouseButton::Left,
                            modifiers: Modifiers::new(),
                        });
                        return;
                    },
                    EventResult::Prevented => {
                        // 阻止默认行为，不调用旧回调，不记录事件
                        return;
                    },
                    EventResult::Continue(_msg) => {
                        // 继续传播——fall through 到旧回调路径
                    },
                }
            }

            // 旧路径：register_interaction 回调（fallback）
            if let Some((_bounds, action, cb)) = self.app.interactions.get_mut(&hit_id) {
                // 交互回调带异常隔离（D1 §11.3）
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    cb(action);
                }));
                if let Err(e) = result {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        *s
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.as_str()
                    } else {
                        "unknown panic"
                    };
                    eprintln!("[rgui] 交互回调 panic (widget={hit_id:?}, action={action}): {msg}");
                }
                // 记录事件
                self.app.events.push(Event::MouseDown {
                    position,
                    button: MouseButton::Left,
                    modifiers: Modifiers::new(),
                });
            }
        }
    }

    /// Skia fallback 切换（D3 §12.5）。
    ///
    /// 当 Vello 渲染失败时，在下一帧边界切换到备用后端。
    /// 通过 RenderBackendFactory 自动选择最佳可用后端
    /// （通常为 SkiaBackend）。
    fn try_fallback_to_skia(&mut self) {
        let params = RenderParams {
            width: self.width.max(1),
            height: self.height.max(1),
            scale_factor: self.scale_factor,
            ..Default::default()
        };
        match RenderBackendFactory::create(&params) {
            Ok(backend) => {
                eprintln!(
                    "[rgui] Vello 渲染失败，在帧边界切换到 {} ({}x{})",
                    backend.backend_name(),
                    self.width,
                    self.height
                );
                self.render_ctx = Some(backend);
            },
            Err(e) => {
                eprintln!("[rgui] fallback 后端创建失败: {e}");
            },
        }
    }

    /// 窗口最小化或 surface 不可用时跳过渲染（D3 §12.4）。
    ///
    /// 当窗口尺寸为 0（最小化状态）或渲染后端不可用时，
    /// 应跳过场景构建与 GPU 提交以节省资源，不视为错误。
    fn should_skip_render(&self) -> bool {
        self.width == 0
            || self.height == 0
            || self
                .render_ctx
                .as_ref()
                .is_none_or(|ctx| !ctx.is_available())
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
            self.app.scale_factor = self.scale_factor;
            self.window = Some(Arc::clone(&window));

            match VelloBackend::new(Arc::clone(&window), w, h) {
                Ok(ctx) => {
                    println!("GPU: Vello (GPU) {w}x{h} (scale: {:.2})", self.scale_factor);
                    self.render_ctx = Some(Box::new(ctx));
                    self.width = w;
                    self.height = h;
                },
                Err(e) => eprintln!("渲染初始化失败: {e}"),
            }

            println!("rgui 窗口已创建: {}", self.app.config.title);
            println!(
                "scale_factor = {:.2}，物理 {}×{} → 逻辑 {:.0}×{:.0}",
                self.scale_factor,
                self.width,
                self.height,
                self.width as f64 / self.scale_factor,
                self.height as f64 / self.scale_factor
            );
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
                // 帧边界 fallback 切换（D3 §12.5）：
                // 上一帧 Vello 渲染失败 → 切换为 Skia
                if self.backend_fallback_pending {
                    self.try_fallback_to_skia();
                    self.backend_fallback_pending = false;
                }
                // 窗口最小化或 surface 不可用时跳过渲染（D3 §12.4）
                if self.should_skip_render() {
                    return;
                }
                if let Some(ref mut ctx) = self.render_ctx {
                    let frame = self.frame_count;
                    // 使用逻辑像素尺寸供组件 paint/measure，物理像素供 RenderBackend。
                    let logical_w = (self.width as f64 / self.scale_factor).max(1.0);
                    let logical_h = (self.height as f64 / self.scale_factor).max(1.0);
                    // 场景构建回调，带异常隔离（D1 §11.3）
                    let scene = if let Some(ref mut view_builder) = self.view_scene_builder {
                        let text_renderer = &self.text_renderer;
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            view_builder(frame, logical_w as u32, logical_h as u32, text_renderer)
                        })) {
                            Ok(scene) => scene,
                            Err(e) => {
                                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                                    *s
                                } else if let Some(s) = e.downcast_ref::<String>() {
                                    s.as_str()
                                } else {
                                    "unknown panic"
                                };
                                eprintln!("[rgui] 视图场景构建 panic (frame={frame}): {msg}");
                                SceneGraph::new(frame)
                            },
                        }
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
                    // 将字形 Atlas 像素数据传入后端（Vello 特定路径）
                    if self.text_renderer.is_dirty() {
                        let (aw, ah) = self.text_renderer.atlas_dimensions();
                        let pixels = self.text_renderer.atlas_pixels();
                        // 仅 VelloBackend 使用 set_atlas_data
                        if let Some(vello) = ctx
                            .as_any_mut()
                            .and_then(|a| a.downcast_mut::<VelloBackend>())
                        {
                            vello.set_atlas_data(aw, ah, &pixels);
                        }
                        self.text_renderer.clear_dirty();
                    }
                    match ctx.render(&scene, &params) {
                        Ok(()) => self.frame_count += 1,
                        Err(e) => {
                            eprintln!("渲染: {e}");
                            // Vello 渲染失败 → 标记帧边界 fallback（D3 §12.5）
                            let is_vello = ctx
                                .as_any_mut()
                                .and_then(|a| a.downcast_mut::<VelloBackend>())
                                .is_some();
                            if is_vello {
                                self.backend_fallback_pending = true;
                            }
                        },
                    }
                }
            },
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                ref mut inner_size_writer,
            } => {
                self.scale_factor = scale_factor;
                self.app.scale_factor = scale_factor;
                println!(
                    "[rgui] DPI 变化: scale_factor = {:.2}，物理 {}×{} → 逻辑 {:.0}×{:.0}",
                    scale_factor,
                    self.width,
                    self.height,
                    self.width as f64 / scale_factor,
                    self.height as f64 / scale_factor
                );
                // 更新物理尺寸，winit 在 DPI 变化后返回新的物理尺寸
                if let Some(window) = &self.window {
                    let new_size = window.inner_size();
                    self.width = new_size.width;
                    self.height = new_size.height;
                }
                // 通知后端 resize（Vello 特定路径）
                if let Some(ref mut ctx) = self.render_ctx {
                    if let Some(vello) = ctx
                        .as_any_mut()
                        .and_then(|a| a.downcast_mut::<VelloBackend>())
                    {
                        vello.resize(self.width, self.height);
                    }
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

    #[test]
    fn fallback_no_window_does_not_panic() {
        // 模拟没有窗口时的 fallback——不应 panic
        let mut handler = AppHandler::new(App::new(AppConfig::default()));
        // 没有设置 render_ctx 和 window，fallback 应优雅处理
        handler.try_fallback_to_skia();
        // 恢复后 render_ctx 为 SkiaBackend
        assert!(handler.render_ctx.is_some());
    }

    #[test]
    fn device_lost_flag_defaults_false() {
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let app = App::new(config);
        let handler = AppHandler::new(app);
        // 初始状态下无 render_ctx，但无论如何不应 panic
        assert!(handler.render_ctx.is_none());
    }

    // ========================================================================
    // R22: 窗口最小化跳过渲染测试（D3 §12.4）
    // ========================================================================

    #[test]
    fn should_skip_render_when_width_zero() {
        // 窗口宽度为 0（最小化）→ 跳过渲染
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.width = 0;
        handler.height = 100;
        assert!(handler.should_skip_render());
    }

    #[test]
    fn should_skip_render_when_height_zero() {
        // 窗口高度为 0（最小化）→ 跳过渲染
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.width = 100;
        handler.height = 0;
        assert!(handler.should_skip_render());
    }

    #[test]
    fn should_skip_render_when_no_render_ctx() {
        // 无渲染上下文 → 跳过渲染
        let config = AppConfig::new().title("Test").window_size(1024.0, 768.0);
        let mut handler = AppHandler::new(App::new(config));
        handler.width = 800;
        handler.height = 600;
        // render_ctx 为 None
        assert!(handler.should_skip_render());
    }

    // ========================================================================
    // WC01: EventResult 接线测试
    // ========================================================================

    #[test]
    fn event_result_handled_stops_old_callback() {
        // EventResult::Handled → 调用 update()，不调用旧回调
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {
            old_called_clone.store(true, Ordering::SeqCst);
        });

        let ctx_received = Arc::new(AtomicBool::new(false));
        let ctx_received_clone = Arc::clone(&ctx_received);
        app.register_widget_instance(widget_id, move |action, ctx| {
            ctx_received_clone.store(true, Ordering::SeqCst);
            assert_eq!(action, "click");
            assert!(ctx.focus.is_none());
            EventResult::Handled
        });

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        // WidgetSpec 路径被调用
        assert!(
            ctx_received.load(Ordering::SeqCst),
            "WidgetSpec handler 应被调用"
        );
        // 旧回调不应被调用
        assert!(!old_called.load(Ordering::SeqCst), "旧回调不应被调用");
        // 事件应被记录
        assert_eq!(handler.app.event_count(), 1);
    }

    #[test]
    fn event_result_continue_falls_through_to_old_callback() {
        // EventResult::Continue → 继续传播，调用旧回调
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |action| {
            assert_eq!(action, "click");
            old_called_clone.store(true, Ordering::SeqCst);
        });

        app.register_widget_instance(widget_id, move |action, _ctx| {
            assert_eq!(action, "click");
            EventResult::Continue("click".to_string())
        });

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        // 旧回调应被调用（Continue 后 fall through）
        assert!(
            old_called.load(Ordering::SeqCst),
            "旧回调应被调用（Continue 后 fall through）"
        );
        assert_eq!(handler.app.event_count(), 1);
    }

    #[test]
    fn event_result_prevented_stops_and_no_event_recorded() {
        // EventResult::Prevented → 不调用旧回调，不记录事件
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {
            old_called_clone.store(true, Ordering::SeqCst);
        });

        app.register_widget_instance(widget_id, move |action, _ctx| {
            assert_eq!(action, "click");
            EventResult::Prevented
        });

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        // 旧回调不应被调用
        assert!(
            !old_called.load(Ordering::SeqCst),
            "旧回调不应被调用（Prevented）"
        );
        // 事件不应被记录
        assert_eq!(handler.app.event_count(), 0);
    }

    #[test]
    fn no_widget_instance_falls_back_to_old_callback() {
        // 未注册 WidgetSpec 实例 → 直接走旧路径
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |action| {
            assert_eq!(action, "click");
            old_called_clone.store(true, Ordering::SeqCst);
        });

        // 不注册 WidgetSpec 实例

        let mut handler = AppHandler::new(app);
        handler.handle_click(Point::new(50.0, 30.0));

        assert!(old_called.load(Ordering::SeqCst), "旧回调应被调用");
        assert_eq!(handler.app.event_count(), 1);
    }

    #[test]
    fn hit_miss_does_nothing() {
        // 点击空白区域 → 无事发生
        let mut app = App::new(AppConfig::default());
        let widget_id = WidgetId::from_u64(1);
        let bounds = Rect::new(10.0, 10.0, 100.0, 40.0);

        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let old_called = Arc::new(AtomicBool::new(false));
        let old_called_clone = Arc::clone(&old_called);
        app.register_interaction(widget_id, bounds, "click", move |_| {
            old_called_clone.store(true, Ordering::SeqCst);
        });
        app.register_widget_instance(widget_id, |_, _| EventResult::Handled);

        let mut handler = AppHandler::new(app);
        // 点击位置在 widget 外部
        handler.handle_click(Point::new(200.0, 200.0));

        assert!(!old_called.load(Ordering::SeqCst));
        assert_eq!(handler.app.event_count(), 0);
    }

    // ========================================================================
    // RG04: .rgui App 集成测试
    // ========================================================================

    #[cfg(feature = "devtools")]
    mod devtools_tests {
        use super::*;

        /// 测试用消息类型。
        #[derive(Debug, Clone, PartialEq)]
        enum TestMsg {}
        impl AppMessage for TestMsg {
            fn message_name(&self) -> &'static str {
                match *self {}
            }
        }

        #[test]
        fn config_rgui_path_builder() {
            let config = AppConfig::new().rgui_path("ui/app.rgui");
            assert!(config.rgui_path.is_some());
            assert_eq!(
                config.rgui_path.unwrap(),
                std::path::PathBuf::from("ui/app.rgui")
            );
        }

        #[test]
        fn config_rgui_path_default_is_none() {
            let config = AppConfig::default();
            assert!(config.rgui_path.is_none());
        }

        #[test]
        fn load_rgui_errors_without_rgui_path() {
            let config = AppConfig::new();
            let mut app = App::new(config);
            let result = app.load_rgui::<TestMsg>();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("未设置 rgui_path"));
        }

        #[test]
        fn load_rgui_with_nonexistent_file() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let non_existent = dir.path().join("no_such.rgui");
            let config = AppConfig::new().rgui_path(&non_existent);
            let mut app = App::new(config);
            let result = app.load_rgui::<TestMsg>();
            assert!(result.is_err());
        }

        #[test]
        fn load_rgui_with_valid_rgui_file_sets_view_scene_builder() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rgui_path = dir.path().join("app.rgui");
            std::fs::write(
                &rgui_path,
                r#"<Column spacing="8"><Label text="Hello"/></Column>"#,
            )
            .expect("写入 .rgui 文件失败");

            let config = AppConfig::new().rgui_path(&rgui_path);
            let mut app = App::new(config);
            let result = app.load_rgui::<TestMsg>();
            assert!(result.is_ok(), "load_rgui 应成功: {result:?}");

            // view_scene_builder 应被设置
            assert!(app.view_scene_builder.is_some());
        }

        #[test]
        fn load_rgui_view_scene_builder_produces_scene() {
            let dir = tempfile::tempdir().expect("创建临时目录失败");
            let rgui_path = dir.path().join("app.rgui");
            std::fs::write(
                &rgui_path,
                r#"<Column spacing="8"><Label text="Hello"/></Column>"#,
            )
            .expect("写入 .rgui 文件失败");

            let config = AppConfig::new().rgui_path(&rgui_path);
            let mut app = App::new(config);
            app.load_rgui::<TestMsg>().expect("load_rgui 应成功");

            // 调用 view_scene_builder 应返回有效的 SceneGraph
            let text_renderer = TextRenderer::new(rgui_render::TextureId(0));
            let scene = app.view_scene_builder.as_mut().unwrap()(1, 800, 600, &text_renderer);
            // 基本断言：场景图应非空（至少有一个图层）
            assert!(!scene.is_empty(), "SceneGraph 应包含图层");
        }
    }
}
