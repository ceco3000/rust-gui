//! 热重载配置——监控目录、debounce 窗口时长、启用的反馈层级。
//!
//! ## 核心类型
//!
//! | 类型 | 职责 |
//! |------|------|
//! | [`HotReloadLayers`] | 按层级开关热重载（样式/结构/Rust/脚本） |
//! | [`HotReloadConfig`] | 聚合配置——监控路径、debounce 时长、层级开关、文件大小上限 |
//!
//! ## 使用示例
//!
//! ```ignore
//! use rgui_devtools::config::HotReloadConfig;
//! use std::path::PathBuf;
//!
//! // 默认配置：监控当前目录，300ms debounce，样式+Rust 层启用
//! let config = HotReloadConfig::default();
//!
//! // 自定义：仅样式热重载
//! let style_only = HotReloadConfig::default()
//!     .with_watch_paths(vec![PathBuf::from("styles/")])
//!     .style_only();
//! ```

use std::path::PathBuf;
use std::time::Duration;

/// 启用的热重载层级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotReloadLayers {
    /// 第 1 层：样式热重载（.rgss 文件，< 200ms）。
    pub style: bool,
    /// 第 2 层：结构热重载（.rgui 文件，< 1s）。
    pub structure: bool,
    /// 第 3 层：Rust 逻辑反馈（快速重启，2-5s）。
    /// 阶段 1 简化版：仅检测变更，不执行重启。
    pub rust: bool,
}

impl Default for HotReloadLayers {
    fn default() -> Self {
        Self {
            style: true,
            structure: false,
            rust: true,
        }
    }
}

/// 热重载管理器配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotReloadConfig {
    /// 监控的项目目录列表。
    pub watch_paths: Vec<PathBuf>,
    /// 文件变更 debounce 时间。
    pub debounce_duration: Duration,
    /// 启用的热重载层级。
    pub layers: HotReloadLayers,
    /// 文件大小警告阈值（字节）。超过此值将发出警告，但不会阻止热重载。
    pub max_file_size: u64,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![PathBuf::from(".")],
            debounce_duration: Duration::from_millis(300),
            layers: HotReloadLayers::default(),
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

impl HotReloadConfig {
    /// 创建新的默认配置。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置监控路径（消耗式构建器版本）。
    #[must_use]
    pub fn with_watch_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.watch_paths = paths;
        self
    }

    /// 设置监控路径（可变引用版本）。
    pub fn set_watch_paths(&mut self, paths: Vec<PathBuf>) {
        self.watch_paths = paths;
    }

    /// 仅监控指定目录的样式文件。
    #[must_use]
    pub fn style_only(mut self) -> Self {
        self.layers = HotReloadLayers {
            style: true,
            structure: false,
            rust: false,
        };
        self
    }

    /// 所有热重载层级全部开启。
    #[must_use]
    pub fn all_layers(mut self) -> Self {
        self.layers = HotReloadLayers {
            style: true,
            structure: true,
            rust: true,
        };
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HotReloadConfig::default();
        assert_eq!(config.watch_paths, vec![PathBuf::from(".")]);
        assert_eq!(config.debounce_duration, Duration::from_millis(300));
        assert!(config.layers.style);
        assert!(!config.layers.structure);
        assert!(config.layers.rust);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_style_only() {
        let config = HotReloadConfig::default().style_only();
        assert!(config.layers.style);
        assert!(!config.layers.structure);
        assert!(!config.layers.rust);
    }

    #[test]
    fn test_all_layers() {
        let config = HotReloadConfig::default().all_layers();
        assert!(config.layers.style);
        assert!(config.layers.structure);
        assert!(config.layers.rust);
    }

    #[test]
    fn test_default_layers_style_enabled() {
        let layers = HotReloadLayers::default();
        assert!(layers.style);
        assert!(!layers.structure);
        assert!(layers.rust);
    }

    #[test]
    fn test_set_watch_paths() {
        let mut config = HotReloadConfig::default();
        config.set_watch_paths(vec![PathBuf::from("/tmp"), PathBuf::from("/src")]);
        assert_eq!(config.watch_paths.len(), 2);
        assert_eq!(config.watch_paths[0], PathBuf::from("/tmp"));
    }
}
