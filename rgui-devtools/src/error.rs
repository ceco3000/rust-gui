//! 开发工具错误类型。
//!
//! 本模块定义 [`DevToolsError`] 枚举，覆盖文件监控、热重载、配置、IPC 及运行时等
//! 全部错误场景。使用 [`thiserror`] 派生 `Display` 和 `Error` trait。
//!
//! ## 错误分类
//!
//! | 变体 | 触发场景 |
//! |------|---------|
//! | [`WatchFailed`](DevToolsError::WatchFailed) | 文件系统监控启动失败或运行时异常 |
//! | [`ReloadFailed`](DevToolsError::ReloadFailed) | 热重载处理过程中出错（携带路径和原因） |
//! | [`ConfigError`](DevToolsError::ConfigError) | 配置校验失败（无效路径、冲突参数等） |
//! | [`IpcError`](DevToolsError::IpcError) | 双进程 IPC 通信错误（阶段 2 激活） |
//! | [`FileNotFound`](DevToolsError::FileNotFound) | 结构热重载中引用的文件不存在（阶段 2 激活） |
//! | [`RuntimeError`](DevToolsError::RuntimeError) | 快速重启过程中发生的通用运行时错误（阶段 2 激活） |
//!
//! `DevToolsError` 标记了 `#[non_exhaustive]`，外部 match 时需处理通配分支
//! 以兼容未来新增的变体。
//!
//! ## 与其他错误的转换
//!
//! - `From<notify::Error>` ➜ [`DevToolsError::WatchFailed`]
//! - `From<DevToolsError>` ➜ [`IpcError`](super::ipc::IpcError)（阶段 2）

use std::path::PathBuf;

/// 开发工具错误枚举。
///
/// 使用 `#[non_exhaustive]` 以兼容未来新增错误变体。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DevToolsError {
    /// 文件监控错误。
    #[error("文件监控失败：{0}")]
    WatchFailed(String),

    /// 热重载处理错误。
    #[error("热重载失败，文件 {path}：{reason}")]
    ReloadFailed {
        /// 出错的文件路径。
        path: PathBuf,
        /// 失败原因。
        reason: String,
    },

    /// 配置错误。
    #[error("配置错误：{0}")]
    ConfigError(String),

    /// IPC 通信错误。
    /// 阶段 2：双进程通信激活时启用。
    #[error("IPC 通信错误：{0}")]
    #[allow(dead_code)]
    IpcError(String),

    /// 文件未找到。
    /// 阶段 2：结构热重载激活时启用。
    #[error("文件未找到：{0}")]
    #[allow(dead_code)]
    FileNotFound(PathBuf),

    /// 运行时错误。
    /// 阶段 2：快速重启机制激活时启用。
    #[error("开发工具运行时错误：{0}")]
    #[allow(dead_code)]
    RuntimeError(String),
}

#[cfg(feature = "notify")]
impl From<notify::Error> for DevToolsError {
    fn from(e: notify::Error) -> Self {
        Self::WatchFailed(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_watch_failed() {
        let err = DevToolsError::WatchFailed("权限不足".into());
        assert!(err.to_string().contains("权限不足"));
    }

    #[test]
    fn test_display_reload_failed() {
        let err = DevToolsError::ReloadFailed {
            path: PathBuf::from("theme.rgss"),
            reason: "语法错误".into(),
        };
        assert!(err.to_string().contains("theme.rgss"));
        assert!(err.to_string().contains("语法错误"));
    }

    #[cfg(feature = "notify")]
    #[test]
    fn test_from_notify_error() {
        let notify_err = notify::Error::generic("test error");
        let dev_err: DevToolsError = notify_err.into();
        assert!(dev_err.to_string().contains("test error"));
    }

    #[test]
    fn test_display_config_error() {
        let err = DevToolsError::ConfigError("无效路径".into());
        assert_eq!(err.to_string(), "配置错误：无效路径");
    }

    #[test]
    fn test_display_ipc_error() {
        let err = DevToolsError::IpcError("连接断开".into());
        assert!(err.to_string().contains("连接断开"));
    }

    #[test]
    fn test_display_file_not_found() {
        let err = DevToolsError::FileNotFound(PathBuf::from("missing.rgss"));
        assert!(err.to_string().contains("missing.rgss"));
    }
}
