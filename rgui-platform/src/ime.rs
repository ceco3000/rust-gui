//! IME（输入法）模块（winit 隔离）。
//! D3 阶段 0：占位类型定义。

/// IME 上下文字段（候选/预编辑区占位）。
#[derive(Debug, Clone, Default)]
pub struct ImeContext {
    /// 窗口 ID。
    pub window_id: Option<rgui_core::id::WindowId>,
}

impl ImeContext {
    /// 构造空 IME 上下文。
    pub fn new() -> Self {
        Self::default()
    }
}

/// IME 事件。
#[derive(Debug, Clone, PartialEq)]
pub enum ImeEvent {
    /// 预编辑更新。
    Preedit(String),
    /// 提交。
    Commit(String),
}
