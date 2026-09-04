//! 组件节点（保留模块，契约 §1.3 F）。

use rgui_core::id::WidgetId;
use rgui_core::view::WidgetView;

/// 组件树节点。
#[derive(Debug, Clone)]
pub struct WidgetNode {
    /// 组件 ID。
    pub id: WidgetId,
    /// 视图占位。
    pub view: WidgetView,
}

impl WidgetNode {
    /// 构造占位节点。
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            view: WidgetView::empty(),
        }
    }
}

impl Default for WidgetNode {
    fn default() -> Self {
        Self::new(WidgetId::new(0))
    }
}
