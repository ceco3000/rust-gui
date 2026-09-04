//! IME（输入法）模块（winit 隔离）。
//!
//! D20：`ImeEvent` 已统一实现于 `input.rs`（4 变体：Enabled/Preedit{text}/Commit{text}/Disabled，
//! 唯一权威定义），此处不再重复定义。仅保留 `ImeContext`（候选/预编辑区占位，供 P1 文本编辑组件接入）。

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
