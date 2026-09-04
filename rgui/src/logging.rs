//! 日志模块（保留，契约 §1.3 F）。D3 阶段 0：占位。

/// 日志级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// 错误。
    Error,
    /// 警告。
    Warn,
    /// 信息。
    Info,
    /// 调试。
    Debug,
}

/// 日志器（D3 占位）。
#[derive(Debug, Default)]
pub struct Logger;

impl Logger {
    /// 记录日志。D3 占位。
    pub fn log(&self, _level: LogLevel, _msg: &str) {
        // todo!("日志实现在实现阶段补全")
    }
}
