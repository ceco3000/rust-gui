//! 测试自动化桩——`InteractionAutomationHarness`（契约 §3.1 模块 5）。
//!
//! 契约 §3.2：必须隔离——放在 `#[cfg(test)]` 或不进入生产公共路径。
//! D3 阶段 0：仅定义 cfg(test) 下的桩结构。测试桩方法（inject_/replay_）实现阶段补全。

/// 测试自动化桩。
///
/// 仅在 `test` 构建下可用（契约 §3.2 隔离要求）。
#[cfg(test)]
#[derive(Debug, Default)]
pub struct InteractionAutomationHarness {
    /// 注入的窗口事件计数占位。
    pub injected: usize,
}

#[cfg(test)]
impl InteractionAutomationHarness {
    /// 构造空桩。
    pub fn new() -> Self {
        Self::default()
    }
}
