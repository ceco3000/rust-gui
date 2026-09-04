//! 窗口模块（winit 隔离）。
//!
//! D8：封装 winit `Window` + `WindowConfig → winit::window::WindowAttributes` 映射，
//! 并提供 [`attributes`] / [`Window`] 等公共 API。窗口**创建**在 winit 事件循环
//! （`ActiveEventLoop::create_window`）内进行，由 `event_loop::run_as` 驱动。

use rgui_core::id::WindowId;

/// winit 窗口类型（re-export，上层无需 `winit::` 前缀）。
pub use winit::window::Window;
/// winit 窗口属性（re-export）。
pub use winit::window::WindowAttributes;

/// 窗口配置。
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// 窗口标题。
    pub title: String,
    /// 逻辑宽度。
    pub width: u32,
    /// 逻辑高度。
    pub height: u32,
}

impl WindowConfig {
    /// 构造默认窗口配置（非零尺寸 300x200，避免窗口零尺寸不可见）。
    pub fn new() -> Self {
        Self {
            title: "rgui".to_string(),
            width: 300,
            height: 200,
        }
    }

    /// 带标题/尺寸的配置。
    pub fn named(mut self, title: impl Into<String>, width: u32, height: u32) -> Self {
        self.title = title.into();
        self.width = width;
        self.height = height;
        self
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 窗口句柄。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowHandle {
    pub window_id: WindowId,
}

impl WindowHandle {
    /// 构造占位句柄。
    pub fn new(window_id: WindowId) -> Self {
        Self { window_id }
    }
}

/// 把 `WindowConfig` 映射为 winit `WindowAttributes`。
pub fn attributes(config: &WindowConfig) -> WindowAttributes {
    WindowAttributes::default()
        .with_title(config.title.clone())
        .with_inner_size(winit::dpi::LogicalSize::new(
            config.width as f64,
            config.height as f64,
        ))
}
