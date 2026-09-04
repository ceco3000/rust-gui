//! 交互命中测试——`InteractionRegion`/`ResolvedHitTest`（契约 §3.1 模块 4）。
//! D3 阶段 0：占位类型定义。

use rgui_core::geometry::Rect;
use rgui_core::id::WidgetId;

/// 交互区域。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InteractionRegion {
    /// 组件 ID。
    pub widget_id: WidgetId,
    /// 命中区域。
    pub bounds: Rect,
}

/// 已解析的命中测试结果。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedHitTest {
    /// 命中的组件 ID。
    pub widget_id: WidgetId,
}

// 命中测试实现（hit_test_*）在实现阶段补全。
