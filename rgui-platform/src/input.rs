//! 输入模块（winit 隔离）。
//! D3 阶段 0：占位类型定义。

/// 输入模态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputModality {
    #[default]
    /// 指针/鼠标。
    Pointer,
    /// 键盘。
    Keyboard,
    /// 触屏。
    Touch,
}

/// 输入事件。
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// 光标移动。
    CursorMoved { x: f32, y: f32 },
    /// 按下。
    Pressed,
    /// 释放。
    Released,
    /// 文本输入。
    Text(String),
}
