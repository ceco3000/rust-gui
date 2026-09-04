//! 交互系统（保留模块，契约 §1.3 F）。与 `interaction.rs` 区分：
//! `interaction.rs` 为命中测试解析，`interactive.rs` 为组件可交互性（点击/悬停回调）。

use rgui_core::id::WidgetId;

/// 交互状态（D3 占位）。
#[derive(Debug, Clone, Default)]
pub struct Interactive {
    /// 是否悬停。
    pub hovered: bool,
}

impl Interactive {
    /// 构造默认交互状态。
    pub fn new() -> Self {
        Self::default()
    }

    /// 绑定组件 ID。D3 占位。
    pub fn for_widget(_id: WidgetId) -> Self {
        Self::default()
    }
}
