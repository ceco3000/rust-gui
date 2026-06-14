//! 开发工具错误类型。

use std::path::PathBuf;

/// 开发工具错误枚举。
#[derive(Debug, thiserror::Error)]
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
    #[error("IPC 通信错误：{0}")]
    #[allow(dead_code)]
    IpcError(String),

    /// 文件未找到。
    #[error("文件未找到：{0}")]
    #[allow(dead_code)]
    FileNotFound(PathBuf),

    /// 运行时错误。
    #[error("开发工具运行时错误：{0}")]
    #[allow(dead_code)]
    RuntimeError(String),
}

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
