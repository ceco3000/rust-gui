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

/// 当前平台窗口 scale_factor（D15：物理→逻辑坐标换算基准；AppRunnerImpl 每事件更新，demo/上层读取使用）。
use std::cell::Cell;
thread_local! {
    static PLATFORM_SCALE: Cell<f64> = Cell::new(1.0);
}

/// 设置当前平台缩放（AppRunnerImpl 在事件处理时用 `window.scale_factor()` 更新）。
pub fn set_platform_scale(scale: f64) {
    PLATFORM_SCALE.with(|c| c.set(scale));
}

/// 读取当前平台缩放（默认 1.0）。
pub fn platform_scale() -> f64 {
    PLATFORM_SCALE.with(|c| c.get())
}

/// 物理像素坐标 → 逻辑坐标（除以 scale_factor）。D15：hit-test/布局用逻辑坐标，避免高分屏/多显示器 DPI 偏移。
pub fn to_logical(physical: (f64, f64), scale: f64) -> (f32, f32) {
    let s = if scale > 0.0 { scale } else { 1.0 };
    ((physical.0 / s) as f32, (physical.1 / s) as f32)
}

/// 取窗口 scale_factor（winit）。
pub fn window_scale(window: &Window) -> f64 {
    window.scale_factor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_logical_divides_by_scale() {
        // Retina(scale=2)：物理 (200,100) → 逻辑 (100,50)
        let (x, y) = to_logical((200.0, 100.0), 2.0);
        assert!((x - 100.0).abs() < 0.01, "x/{x}");
        assert!((y - 50.0).abs() < 0.01, "y/{y}");
    }

    #[test]
    fn to_logical_identity_at_scale_one() {
        let (x, y) = to_logical((37.0, 42.0), 1.0);
        assert!((x - 37.0).abs() < 0.01);
        assert!((y - 42.0).abs() < 0.01);
    }

    #[test]
    fn platform_scale_defaults_to_one_and_can_set() {
        assert_eq!(platform_scale(), 1.0);
        set_platform_scale(2.0);
        assert_eq!(platform_scale(), 2.0);
        set_platform_scale(1.0); // 复位
    }
}
