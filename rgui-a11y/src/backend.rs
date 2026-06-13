//! 无障碍后端 trait（D6）。
//!
//! 占位模块——完整 AccessKit 集成在后续实现。

use crate::tree::TreeUpdate;
use rgui_core::id::WidgetId;

/// 无障碍后端操作。
pub trait A11yBackend: Send + Sync {
    /// 推送树更新到平台无障碍 API。
    fn push_update(&mut self, update: TreeUpdate);

    /// 处理平台无障碍 action。
    fn handle_action(&mut self, widget_id: WidgetId, action: &str) -> bool;

    /// 后端名称。
    fn name(&self) -> &'static str;
}

/// 空后端（无操作）。
pub struct NullBackend;

impl A11yBackend for NullBackend {
    fn push_update(&mut self, _update: TreeUpdate) {}
    fn handle_action(&mut self, _widget_id: WidgetId, _action: &str) -> bool { false }
    fn name(&self) -> &'static str { "null" }
}
