//! 焦点管理模块（契约 §4 R1：焦点管理原生在此，保留原样）。
//! D3 阶段 0：占位类型定义。

use crate::input::InputModality;

/// 焦点管理器。
#[derive(Debug, Default)]
pub struct FocusManager {
    /// 当前焦点组件 ID 占位。
    pub focused: Option<rgui_core::id::WidgetId>,
}

impl FocusManager {
    /// 构造空焦点管理器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置当前输入模态。D3 占位。
    pub fn set_modality(&mut self, _modality: InputModality) {
        // todo!("模态切换在实现阶段补全")
    }

    /// 设置焦点。D3 占位。
    pub fn set_focus(&mut self, widget_id: rgui_core::id::WidgetId) {
        self.focused = Some(widget_id);
    }
}
