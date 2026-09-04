//! 启动协调 / AppConfig / App（greenfield §B.5：极薄启动协调，≤200 行）。
//!
//! D9：统一入口 `App::run`——用户通过 `App::run(config, widget, state, mapper)` 启动
//! 窗口 + 事件循环 + 渲染 + 交互，内部走 `rgui_platform` + `rgui_render`。
//!
//! 窗口后端（winit/vello surface）经 `window` feature 门控：默认（无 window）仅提供
//! `AppConfig`/`App` 容器，`App::run` 仅在启用 `window` 时可用。

/// 应用配置。
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 应用名。
    pub app_name: String,
    /// 窗口标题。
    pub window_title: String,
    /// 逻辑宽度。
    pub width: u32,
    /// 逻辑高度。
    pub height: u32,
}

impl AppConfig {
    /// 构造默认应用配置（非零窗口尺寸）。
    pub fn new() -> Self {
        Self {
            app_name: "rgui".to_string(),
            window_title: "rgui".to_string(),
            width: 300,
            height: 200,
        }
    }

    /// 设置窗口标题。
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = title.into();
        self
    }

    /// 设置窗口尺寸。
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 应用（门面容器）——极薄启动协调。
#[derive(Debug, Default)]
pub struct App {
    /// 配置。
    pub config: AppConfig,
}

impl App {
    /// 构造应用。
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
}

// ===== 窗口后端（仅 `window` feature）=====

#[cfg(feature = "window")]
mod backend {
    use std::sync::Arc;

    use rgui_core::context::{UpdateContext, ViewContext};
    use rgui_core::coordinator::Coordinator;
    use rgui_core::traits::WidgetSpec;
    use rgui_render::scene_graph::SceneGraph;
    use rgui_render::vello::VelloBackend;
    use rgui_render::GpuSurface;
    use rgui_platform::AppRunner;
    use rgui_platform::event_loop::{run_as_with_config, WindowEvent};
    use rgui_platform::window::{Window, WindowConfig};

    use super::{App, AppConfig};

    impl App {
        /// 统一入口：启动窗口 + 事件循环 + 渲染 + 交互。
        ///
        /// - `config`：应用配置（窗口标题/尺寸）。
        /// - `widget`：`WidgetSpec` 组件。
        /// - `state`：组件初始状态。
        /// - `mapper`：把窗口事件（点击/键盘）映射为组件消息（无则 `None`）。
        ///
        /// 内部：`Coordinator` 驱动组件 view/update；`platform` 驱动窗口/事件循环；`render` 渲染 surface。
        #[allow(clippy::type_complexity)]
        pub fn run<W, F>(
            config: AppConfig,
            widget: W,
            state: W::State,
            mapper: F,
        ) -> Result<(), Box<dyn std::error::Error>>
        where
            W: WidgetSpec + 'static,
            F: FnMut(&WindowEvent) -> Option<W::Message> + 'static,
        {
            // 应用配置 → 平台窗口配置（非零尺寸）
            let wc = WindowConfig {
                title: config.window_title.clone(),
                width: config.width,
                height: config.height,
            };
            run_as_with_config(AppRunnerImpl::new(widget, state, mapper), wc)?;
            Ok(())
        }
    }

    /// 内部 `platform::AppRunner` 实现：驱动 Coordinator + surface 渲染。
    struct AppRunnerImpl<W: WidgetSpec> {
        coordinator: Coordinator<W>,
        backend: Option<VelloBackend>,
        surface: Option<GpuSurface<'static>>,
        mapper: Box<dyn FnMut(&WindowEvent) -> Option<W::Message>>,
    }

    impl<W: WidgetSpec + 'static> AppRunnerImpl<W> {
        fn new<F: FnMut(&WindowEvent) -> Option<W::Message> + 'static>(
            widget: W,
            state: W::State,
            mapper: F,
        ) -> Self {
            Self {
                coordinator: Coordinator::new(widget, state),
                backend: None,
                surface: None,
                mapper: Box::new(mapper),
            }
        }
    }

    impl<W: WidgetSpec + 'static> AppRunner for AppRunnerImpl<W> {
        fn init(&mut self, window: Arc<Window>) {
            let backend = VelloBackend::new().expect("vello backend");
            let surface = backend.create_surface(window.clone()).expect("create surface");
            window.request_redraw();
            self.backend = Some(backend);
            self.surface = Some(surface);
        }

        fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
            // D15：把窗口 scale_factor 注入平台层（demo/上层读 platform_scale() 做物理→逻辑坐标换算）
            rgui_platform::window::set_platform_scale(window.scale_factor());
            if let Some(msg) = (self.mapper)(event) {
                let mut ctx = UpdateContext::default();
                self.coordinator.dispatch(msg, &mut ctx);
                true // 状态已更新，需重绘
            } else {
                false
            }
        }

        fn draw(&mut self, window: &Window) {
            let (Some(backend), Some(surface)) = (&mut self.backend, &self.surface) else {
                return;
            };
            let size = window.inner_size();
            let view_tree = self.coordinator.current_view(&ViewContext::default());
            let graph = SceneGraph::from_view(&view_tree);
            if let Err(e) = backend.render_surface(surface, &graph, size.width, size.height) {
                eprintln!("render error: {e}");
            }
        }
    }
}
