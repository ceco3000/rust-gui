//! .rgui 热重载管理器——监控 `.rgui` 文件，变更时自动重新解析为 WidgetView 树。
//!
//! ## 设计
//!
//! `RguiHotReload` 封装了文件监控（[`FileWatcher`]）和 `.rgui` 解析（[`parse_rgui_file`]），
//! 提供简单的轮询接口：在主循环中调用 [`check_and_reload`](RguiHotReload::check_and_reload)，
//! 若 `.rgui` 文件有变更则返回新的 [`WidgetView`]。
//!
//! ### 与 FileWatcher 的关系
//!
//! `RguiHotReload` 内部持有独立的 [`FileWatcher`]，监控包含 `.rgui` 文件的目录。
//! debounce 窗口通过 [`HotReloadConfig::debounce_duration`] 控制（默认 300ms），
//! 满足 `< 1s` 的 UI 更新延迟要求。
//!
//! ### 使用示例
//!
//! ```ignore
//! use rgui_devtools::rgui_hot_reload::RguiHotReload;
//! use rgui_devtools::config::HotReloadConfig;
//!
//! let config = HotReloadConfig::default();
//! let mut reloader = RguiHotReload::<MyMsg>::new(&config, "ui/app.rgui")?;
//!
//! // 在主循环中
//! loop {
//!     if let Some(new_view) = reloader.check_and_reload()? {
//!         // 差分并更新 UI
//!         app.update_root_view(new_view);
//!     }
//!     // ... 渲染帧
//! }
//! ```
//!
//! 设计源自 D8 RG03、D7 §4。

use std::path::{Path, PathBuf};

use rgui_core::AppMessage;
use rgui_core::view::WidgetView;

use crate::config::HotReloadConfig;
use crate::rgui_parser::{RguiParseError, parse_rgui_file};
use crate::watcher::FileWatcher;

/// .rgui 热重载错误。
#[derive(Debug)]
#[non_exhaustive]
pub enum RguiHotReloadError {
    /// .rgui 解析失败。
    Parse(RguiParseError),
    /// 文件监控或 I/O 错误。
    Watch(String),
}

impl std::fmt::Display for RguiHotReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, ".rgui 解析失败: {e}"),
            Self::Watch(msg) => write!(f, ".rgui 热重载监控失败: {msg}"),
        }
    }
}

impl std::error::Error for RguiHotReloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Watch(_) => None,
        }
    }
}

impl From<RguiParseError> for RguiHotReloadError {
    fn from(e: RguiParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<crate::error::DevToolsError> for RguiHotReloadError {
    fn from(e: crate::error::DevToolsError) -> Self {
        Self::Watch(e.to_string())
    }
}

/// .rgui 热重载管理器。
///
/// 监控指定的 `.rgui` 文件，检测到变更时自动重新解析为 WidgetView 树。
/// 与现有 [`FileWatcher`] 集成，复用 debounce 机制。
///
/// # 类型参数
///
/// * `M` - 应用消息类型（传递给 [`parse_rgui_file`]）。
pub struct RguiHotReload<M: AppMessage> {
    /// 文件监控器。
    watcher: FileWatcher,
    /// 被监控的 .rgui 文件路径（用户传入的原始路径）。
    rgui_path: PathBuf,
    /// 规范化的 .rgui 文件路径（用于与 notify 事件路径比较）。
    canonical_rgui_path: PathBuf,
    /// 监控目录的规范化路径（.rgui 文件所在目录）。
    /// 用于检测目录内任意 .rgui/.rhai 文件的创建/修改事件。
    watch_dir: PathBuf,
    /// 当前 WidgetView 缓存——用于快速返回最新解析结果。
    current_view: WidgetView<M>,
}

impl<M: AppMessage> RguiHotReload<M> {
    /// 创建新的 .rgui 热重载管理器。
    ///
    /// 创建时立即解析一次 .rgui 文件以填充初始视图。
    ///
    /// # 参数
    ///
    /// * `config` - 热重载配置（`watch_paths` 需包含 `rgui_path` 所在目录）
    /// * `rgui_path` - .rgui 文件路径
    ///
    /// # 错误
    ///
    /// * 文件监控创建失败
    /// * .rgui 文件读取或解析失败
    pub fn new(
        config: &HotReloadConfig,
        rgui_path: impl AsRef<Path>,
    ) -> Result<Self, RguiHotReloadError> {
        let rgui_path = rgui_path.as_ref().to_path_buf();
        // 规范化路径以匹配 notify 事件路径（macOS 上 /tmp → /private/tmp）
        let canonical_rgui_path = std::fs::canonicalize(&rgui_path).unwrap_or_else(|e| {
            eprintln!("[rgui] RguiHotReload: canonicalize({rgui_path:?}) 失败: {e}，使用原始路径");
            rgui_path.clone()
        });
        let watcher = FileWatcher::new(config)?;
        let current_view = parse_rgui_file_from_path(&rgui_path)?;
        // 规范化监控目录路径（.rgui 父目录）
        let watch_dir = canonical_rgui_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(Self {
            watcher,
            rgui_path,
            canonical_rgui_path,
            watch_dir,
            current_view,
        })
    }

    /// 检查文件变更，若有则重新解析。
    ///
    /// 在主循环中调用。debounce 窗口由配置中的 [`HotReloadConfig::debounce_duration`]
    /// 控制（默认 300ms）。
    ///
    /// # 返回值
    ///
    /// * `Ok(Some(view))` - `.rgui` 文件已变更，返回新的 `WidgetView`
    /// * `Ok(None)` - 无变更
    /// * `Err(_)` - 解析或监控错误
    pub fn check_and_reload(&mut self) -> Result<Option<WidgetView<M>>, RguiHotReloadError> {
        let changes = self.watcher.check_changes();
        let needs_reload = changes.iter().any(|c| {
            path_matches(&c.path, &self.canonical_rgui_path)
                || is_sibling_rgui_or_rhai(&c.path, &self.watch_dir, &self.canonical_rgui_path)
        });

        if needs_reload {
            let new_view = parse_rgui_file_from_path(&self.rgui_path)?;
            self.current_view = new_view.clone();
            Ok(Some(new_view))
        } else {
            Ok(None)
        }
    }

    /// 获取当前缓存的 WidgetView（不触发重解析）。
    #[must_use]
    pub fn current_view(&self) -> &WidgetView<M> {
        &self.current_view
    }

    /// 强制重新解析并刷新（忽略 debounce 窗口）。
    ///
    /// 调用 [`FileWatcher::flush`] 获取所有待处理事件，
    /// 若 .rgui 文件有变更则立即重新解析。
    pub fn flush_and_reload(&mut self) -> Result<Option<WidgetView<M>>, RguiHotReloadError> {
        let changes = self.watcher.flush();
        let needs_reload = changes.iter().any(|c| {
            path_matches(&c.path, &self.canonical_rgui_path)
                || is_sibling_rgui_or_rhai(&c.path, &self.watch_dir, &self.canonical_rgui_path)
        });

        if needs_reload {
            let new_view = parse_rgui_file_from_path(&self.rgui_path)?;
            self.current_view = new_view.clone();
            Ok(Some(new_view))
        } else {
            Ok(None)
        }
    }
}

impl<M: AppMessage> std::fmt::Debug for RguiHotReload<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RguiHotReload")
            .field("rgui_path", &self.rgui_path)
            .field("widget_type", &self.current_view.widget_type)
            .finish_non_exhaustive()
    }
}

/// 从文件路径解析 .rgui。包装 [`parse_rgui_file`] 以处理路径不存在的情况。
fn parse_rgui_file_from_path<M: AppMessage>(
    path: &Path,
) -> Result<WidgetView<M>, RguiHotReloadError> {
    if !path.exists() {
        return Err(RguiHotReloadError::Watch(format!(
            ".rgui 文件不存在: {}",
            path.display()
        )));
    }
    Ok(parse_rgui_file(path)?)
}

/// 比较 notify 事件路径与规范化的 .rgui 文件路径。
///
/// macOS 上 `/tmp` 是 `/private/tmp` 的符号链接，`fs::canonicalize` 返回真实路径。
/// 同时回退到原始路径比较以处理不存在的文件。
fn path_matches(event_path: &Path, canonical_target: &Path) -> bool {
    // 首先直接比较
    if event_path == canonical_target {
        return true;
    }
    // 规范化事件路径再比较
    if let Ok(canonical_event) = std::fs::canonicalize(event_path) {
        return canonical_event == canonical_target;
    }
    false
}

/// 检测事件路径是否为监控目录内的 .rgui/.rhai 文件（排除主 .rgui 文件自身）。
///
/// 用于 T205：当目录内新增 `.rgui`/`.rhai` 文件时，触发重新解析主 `.rgui`，
/// 以执行 Tier 2 组件扫描（`mark_tier2_nodes`）。
fn is_sibling_rgui_or_rhai(event_path: &Path, watch_dir: &Path, main_rgui: &Path) -> bool {
    // 排除主 .rgui 文件自身（由 path_matches 单独处理）
    if path_matches(event_path, main_rgui) {
        return false;
    }

    // 检查扩展名
    let is_rgui_or_rhai = event_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "rgui" | "rhai"))
        .unwrap_or(false);

    if !is_rgui_or_rhai {
        return false;
    }

    // 规范化事件路径后检查是否在监控目录内
    if let Ok(canonical_event) = std::fs::canonicalize(event_path) {
        canonical_event.parent().map_or(false, |p| p == watch_dir)
    } else {
        // 无法规范化时，直接比较父目录
        event_path.parent().map_or(false, |p| p == watch_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    /// 测试用消息类型。
    #[derive(Debug, Clone)]
    struct TestMsg;

    impl AppMessage for TestMsg {
        fn message_name(&self) -> &'static str {
            "TestMsg"
        }
    }

    /// 创建一个包含简单 .rgui 文件的临时目录和配置。
    fn setup_test_env(rgui_content: &str) -> (tempfile::TempDir, PathBuf, HotReloadConfig) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let rgui_path = dir.path().join("test.rgui");
        std::fs::write(&rgui_path, rgui_content).expect("写入 .rgui 文件失败");

        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };

        (dir, rgui_path, config)
    }

    // === 创建测试 ===

    #[test]
    fn test_new_creates_successfully() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Hello"/>"#);
        let result = RguiHotReload::<TestMsg>::new(&config, &rgui_path);
        assert!(result.is_ok(), "应成功创建 RguiHotReload");
    }

    #[test]
    fn test_new_preserves_initial_view() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Hello"/>"#);
        let reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");
        let view = reloader.current_view();
        assert_eq!(view.widget_type, "Label");
        assert_eq!(view.props.len(), 1);
    }

    #[test]
    fn test_new_file_not_found() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };
        let non_existent = dir.path().join("no_such_file.rgui");
        let result = RguiHotReload::<TestMsg>::new(&config, &non_existent);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("不存在"), "错误应提及文件不存在: {msg}");
    }

    // === 重载测试 ===

    #[test]
    fn test_check_and_reload_returns_none_without_changes() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Hello"/>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "无变更应返回 None");
    }

    #[test]
    fn test_check_and_reload_detects_change() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Hello"/>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件
        std::fs::write(&rgui_path, r#"<Button label="Click"/>"#).expect("写入 .rgui 文件失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_ok());
        let maybe_view = result.unwrap();
        assert!(maybe_view.is_some(), "文件变更应返回新 WidgetView");
        let new_view = maybe_view.unwrap();
        assert_eq!(new_view.widget_type, "Button");
    }

    #[test]
    fn test_check_and_reload_updates_cache() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Before"/>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件
        std::fs::write(
            &rgui_path,
            r#"<Column spacing="8"><Label text="After"/></Column>"#,
        )
        .expect("写入 .rgui 文件失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some(), "文件变更应返回新视图");

        // 缓存应已更新
        let cached = reloader.current_view();
        assert_eq!(cached.widget_type, "Column");
        assert!(!cached.children.is_empty());
    }

    // === flush_and_reload 测试 ===

    #[test]
    fn test_flush_and_reload_ignores_debounce() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Original"/>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件，立即 flush（不等 debounce）
        std::fs::write(&rgui_path, r#"<Label text="Flushed"/>"#).expect("写入 .rgui 文件失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.flush_and_reload();
        assert!(result.is_ok());
        let maybe_view = result.unwrap();
        assert!(maybe_view.is_some(), "flush 应忽略 debounce 立即返回");
        let new_view = maybe_view.unwrap();
        assert_eq!(new_view.widget_type, "Label");

        // 验证缓存更新
        let cached = reloader.current_view();
        assert_eq!(cached.props.len(), 1);
    }

    // === 错误处理测试 ===

    #[test]
    fn test_reload_malformed_rgui() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Valid"/>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 写入语法错误的 .rgui（未闭合的标签）
        std::fs::write(&rgui_path, r#"<Column><Label text="Oops""#).expect("写入 .rgui 文件失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_err(), "畸形 .rgui 应返回错误");
    }

    // === 错误类型测试 ===

    #[test]
    fn test_error_display_parse() {
        let parse_err = RguiParseError::ParseError {
            line: 2,
            col: 1,
            message: "意外".into(),
        };
        let err = RguiHotReloadError::from(parse_err);
        let msg = err.to_string();
        assert!(msg.contains(".rgui 解析失败"));
        assert!(msg.contains("意外"));
    }

    #[test]
    fn test_error_display_watch() {
        let err = RguiHotReloadError::Watch("目录不存在".into());
        let msg = err.to_string();
        assert!(msg.contains(".rgui 热重载监控失败"));
        assert!(msg.contains("目录不存在"));
    }

    #[test]
    fn test_error_from_rgui_parse_error() {
        let parse_err = RguiParseError::ParseError {
            line: 1,
            col: 1,
            message: "空".into(),
        };
        let err: RguiHotReloadError = parse_err.into();
        assert!(matches!(err, RguiHotReloadError::Parse(_)));
    }

    // === Debug trait 测试 ===

    #[test]
    fn test_debug_output_contains_fields() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Label text="Debug"/>"#);
        let reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        let debug_str = format!("{reloader:?}");
        assert!(debug_str.contains("RguiHotReload"));
        assert!(debug_str.contains("widget_type"));
    }

    // === T205: 目录级创建事件检测 ===

    #[test]
    fn test_detect_new_rgui_rhai_file_pair_in_directory() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Column><Card title="Test"/></Column>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 确认初始无变更
        let result = reloader.check_and_reload().unwrap();
        assert!(result.is_none(), "初始应无变更");

        // 创建 card.rgui + card.rhai 文件对
        let card_rgui = _dir.path().join("card.rgui");
        let card_rhai = _dir.path().join("card.rhai");
        std::fs::write(&card_rgui, r#"<Label text="Card"/>"#).expect("写入 card.rgui 失败");
        std::fs::write(&card_rhai, "// Card paint script").expect("写入 card.rhai 失败");

        // 等待文件系统事件传播 + debounce 窗口
        std::thread::sleep(Duration::from_millis(200));

        // T205: 新增 .rgui+.rhai 文件 → notify 检测 → 自动注册 → 父组件刷新
        let result = reloader.check_and_reload().unwrap();
        assert!(result.is_some(), "新增 .rgui+.rhai 文件对应触发重新解析");
    }

    #[test]
    fn test_detect_new_rgui_only_no_trigger() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Column><Label text="Only"/></Column>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 仅创建 .rgui 文件（无 .rhai 配对），仍应触发检测
        let new_rgui = _dir.path().join("other.rgui");
        std::fs::write(&new_rgui, r#"<Label text="Other"/>"#).expect("写入 other.rgui 失败");

        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload().unwrap();
        assert!(
            result.is_some(),
            "新增 .rgui 文件（即使无配对 .rhai）应触发重新解析以扫描 Tier 2"
        );
    }

    #[test]
    fn test_detect_new_rhai_only_triggers_reparse() {
        let (_dir, rgui_path, config) = setup_test_env(r#"<Column><Card title="Test"/></Column>"#);
        let mut reloader =
            RguiHotReload::<TestMsg>::new(&config, &rgui_path).expect("创建 RguiHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 仅创建 .rhai 文件（配对已有的 card.rgui 不存在），仍应触发
        let new_rhai = _dir.path().join("widget.rhai");
        std::fs::write(&new_rhai, "// Widget paint").expect("写入 widget.rhai 失败");

        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload().unwrap();
        assert!(
            result.is_some(),
            "新增 .rhai 文件应触发重新解析以扫描 Tier 2 配对"
        );
    }
}
