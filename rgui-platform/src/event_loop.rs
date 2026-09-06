//! 事件循环模块（winit 隔离）——D8 收敛为 platform 公共 API。
//!
//! 封装 winit `EventLoop` + `ControlFlow::Poll`（持续重绘，首帧不依赖 focus），
//! 通过 [`App`] trait 让上层（facade/示例）提供窗口初始化、事件处理、绘制回调。
//! **上层不再直接引用 `winit::`**。

// re-export winit 公共类型（上层用这些类型，无需 `winit::` 前缀）
pub use winit::dpi;
pub use winit::error::EventLoopError;
pub use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
pub use winit::event_loop::{ActiveEventLoop, ControlFlow};
pub use winit::keyboard::{KeyCode, PhysicalKey};
pub use winit::window::WindowAttributes;

use crate::input::{ImeEvent, InputEvent};
use winit::event::Ime;

/// winit 事件 → rgui `InputEvent`（D20 真实驱动转换：光标/按下/释放/文本，供上层文本/指针输入接入）。
///
/// 返回 `None` 表示该 winit 事件不映射为 rgui 输入事件（如窗口焦点/IME 提交等）。
pub fn to_input_event(event: &WindowEvent) -> Option<InputEvent> {
    match event {
        WindowEvent::CursorMoved { position, .. } => Some(InputEvent::CursorMoved {
            x: position.x as f32,
            y: position.y as f32,
        }),
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            ..
        } => Some(InputEvent::Pressed),
        WindowEvent::MouseInput {
            state: ElementState::Released,
            ..
        } => Some(InputEvent::Released),
        WindowEvent::KeyboardInput { event: ke, .. } => {
            ke.text.clone().map(|t| InputEvent::Text(t.to_string()))
        }
        _ => None,
    }
}

/// winit IME 事件 → rgui `ImeEvent`（D20 Preedit/Commit 真实链路；组合输入事件流）。
pub fn to_ime_event(event: &WindowEvent) -> Option<ImeEvent> {
    match event {
        WindowEvent::Ime(Ime::Enabled) => Some(ImeEvent::Enabled),
        WindowEvent::Ime(Ime::Preedit(text, _)) => Some(ImeEvent::Preedit { text: text.clone() }),
        WindowEvent::Ime(Ime::Commit(text)) => Some(ImeEvent::Commit { text: text.clone() }),
        WindowEvent::Ime(Ime::Disabled) => Some(ImeEvent::Disabled),
        _ => None,
    }
}

/// winit 原生事件循环（`EventLoop<()>`）。
pub type NativeEventLoop = winit::event_loop::EventLoop<()>;

/// 应用回调 trait：由上层实现（facade/示例），platform 驱动。
///
/// - [`App::init`]：窗口创建后调用一次（在此建 surface/渲染后端）。`window` 为 `Arc`，
///   上层可 clone 持有以建 `'static` surface。
/// - [`App::event`]：收到窗口事件（点击/键盘/关闭/尺寸）时调用。
/// - [`App::draw`]：每帧渲染（`RedrawRequested` / `about_to_wait` 主动请求）。
pub trait App: 'static {
    /// 窗口初始化（创建 surface/渲染后端）。首次窗口出现前调用。
    fn init(&mut self, window: std::sync::Arc<crate::window::Window>);
    /// 窗口事件处理。返回 `true` 表示需要重绘（状态已变更），返回 `false` 则表示无需重绘。
    fn event(&mut self, window: &crate::window::Window, event: &WindowEvent) -> bool;
    /// 每帧绘制（渲染 + 呈现）。
    fn draw(&mut self, window: &crate::window::Window);
}

/// 创建 winit 事件循环。
pub fn build() -> Result<NativeEventLoop, EventLoopError> {
    winit::event_loop::EventLoop::new()
    // EventLoop::<()> 泛型
}

/// 运行事件循环（驱动 [`App`]，默认 300x200 窗口配置）。
///
/// 内部设置 `ControlFlow::Poll`，并在 `about_to_wait` 里每帧主动 `request_redraw`，
/// 保证窗口**弹出即渲染并呈现组件**（不依赖前台 focus/系统 RedrawRequested 调度）。
pub fn run_as<A: App>(app: A) -> Result<(), EventLoopError> {
    run_as_with_config(app, crate::window::WindowConfig::new())
}

/// 运行事件循环（驱动 [`App`]），使用指定窗口配置（尺寸 >0，避免零尺寸不可见）。
///
/// 内部设置 `ControlFlow::Poll` + `about_to_wait` 每帧 `request_redraw`（首帧稳定渲染）。
pub fn run_as_with_config<A: App>(
    mut app: A,
    config: crate::window::WindowConfig,
) -> Result<(), EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    // D9(c) 按需重绘：Wait 空闲休眠（CPU 低），仅首帧/事件变更（resumed request_redraw /
    // dirty event → request_redraw）触发重绘。about_to_wait 仅在 !has_drawn || pending 时再请求。
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut Runner {
        app: &mut app,
        window: None,
        config,
        has_drawn: false,
        pending: true,
    })?;
    Ok(())
}

/// 内部 winit `ApplicationHandler` 桥接。
struct Runner<'a, A: App> {
    app: &'a mut A,
    window: Option<std::sync::Arc<crate::window::Window>>,
    config: crate::window::WindowConfig,
    /// 首帧是否已绘制（用于首帧兜底重绘）。
    has_drawn: bool,
    /// 是否有待重绘请求（事件变更后置位）。
    pending: bool,
}

impl<'a, A: App> winit::application::ApplicationHandler for Runner<'a, A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = crate::window::attributes(&self.config);
        let window = event_loop
            .create_window(attrs)
            .expect("create platform window");
        let window = std::sync::Arc::new(window);
        tracing::info!(target: "rgui_platform", "window_created");
        self.app.init(std::sync::Arc::clone(&window));
        // 首帧：主动请求一次重绘（窗口弹出即渲染组件）
        window.request_redraw();
        self.pending = true;
        self.has_drawn = false;
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(window) = &self.window {
            match &event {
                WindowEvent::CloseRequested => {
                    tracing::info!(target: "rgui_platform", "window_closed");
                    _event_loop.exit();
                    return;
                }
                _ => {}
            }
            // 事件处理：若状态变更（返回 true）则置 pending，并立即请求重绘
            let dirty = self.app.event(window, &event);
            if dirty {
                self.pending = true;
                window.request_redraw();
            }
            if matches!(event, WindowEvent::RedrawRequested) {
                self.app.draw(window);
                self.has_drawn = true;
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 按需重绘：非首帧且无变更请求时跳过（空闲 CPU 低）。
        if let Some(window) = &self.window {
            if !self.has_drawn || self.pending {
                window.request_redraw();
                self.pending = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{ImeEvent, InputEvent};

    #[test]
    fn ime_commit_maps_to_ime_event() {
        let e = WindowEvent::Ime(Ime::Commit("é".to_string()));
        assert_eq!(
            to_ime_event(&e),
            Some(ImeEvent::Commit {
                text: "é".to_string()
            })
        );
    }

    #[test]
    fn ime_preedit_maps_to_ime_event() {
        let e = WindowEvent::Ime(Ime::Preedit("啊".to_string(), Some((0, 1))));
        assert_eq!(
            to_ime_event(&e),
            Some(ImeEvent::Preedit {
                text: "啊".to_string()
            })
        );
    }

    #[test]
    fn ime_enabled_disabled_map_to_ime_event() {
        assert_eq!(
            to_ime_event(&WindowEvent::Ime(Ime::Enabled)),
            Some(ImeEvent::Enabled)
        );
        assert_eq!(
            to_ime_event(&WindowEvent::Ime(Ime::Disabled)),
            Some(ImeEvent::Disabled)
        );
    }

    #[test]
    fn ime_event_does_not_map_to_input_event() {
        // IME 提交事件不映射为 rgui `InputEvent`（走 ImeEvent 通道）
        let e = WindowEvent::Ime(Ime::Commit("x".to_string()));
        assert_eq!(to_input_event(&e), None);
    }
}
