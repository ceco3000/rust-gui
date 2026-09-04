//! 错误边界模块（保留，契约 §1.3 F）。D3 阶段 0：占位。

use std::any::Any;

/// 错误上抛处理结果。
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorBoundaryOutcome {
    /// 已恢复。
    Recovered,
    /// 终止。
    Fatal,
}

/// 错误边界（捕获组件异常，D3 占位）。
#[derive(Debug, Default)]
pub struct ErrorBoundary;

impl ErrorBoundary {
    /// 捕获异常。D3 占位。
    pub fn catch(&self, _err: Box<dyn Any + Send>) -> ErrorBoundaryOutcome {
        // todo!("错误边界实现在实现阶段补全")
        ErrorBoundaryOutcome::Recovered
    }
}
