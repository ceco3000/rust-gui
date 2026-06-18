//! HTML 热重载管理器——监控 `.html` 文件，变更时自动重新解析为 WidgetView 树。
//!
//! ## 设计
//!
//! `HtmlHotReload` 封装了文件监控（[`FileWatcher`]）和 HTML 解析（[`parse_html_file`]），
//! 提供简单的轮询接口：在主循环中调用 [`check_and_reload`](HtmlHotReload::check_and_reload)，
//! 若 HTML 文件有变更则返回新的 [`WidgetView`]。
//!
//! ### 与 FileWatcher 的关系
//!
//! `HtmlHotReload` 内部持有独立的 [`FileWatcher`]，监控包含 HTML 文件的目录。
//! debounce 窗口通过 [`HotReloadConfig::debounce_duration`] 控制（默认 300ms），
//! 满足 `< 200ms` 的 UI 更新延迟要求。
//!
//! ### 使用示例
//!
//! ```ignore
//! use rgui_devtools::html_hot_reload::HtmlHotReload;
//! use rgui_devtools::config::HotReloadConfig;
//!
//! let config = HotReloadConfig::default();
//! let mut reloader = HtmlHotReload::<MyMsg>::new(&config, "ui/app.html")?;
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
//! 设计源自 D8 §8.6b H06、D7 §2。

use std::path::{Path, PathBuf};

use rgui_core::AppMessage;
use rgui_core::view::WidgetView;

use crate::config::HotReloadConfig;
use crate::html_reload::{HtmlParseError, parse_html_file};
use crate::watcher::FileWatcher;

/// HTML 热重载错误。
#[derive(Debug)]
#[non_exhaustive]
pub enum HtmlHotReloadError {
    /// HTML 解析失败。
    Parse(HtmlParseError),
    /// 文件监控或 I/O 错误。
    Watch(String),
}

impl std::fmt::Display for HtmlHotReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "HTML 解析失败: {e}"),
            Self::Watch(msg) => write!(f, "HTML 热重载监控失败: {msg}"),
        }
    }
}

impl std::error::Error for HtmlHotReloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Watch(_) => None,
        }
    }
}

impl From<HtmlParseError> for HtmlHotReloadError {
    fn from(e: HtmlParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<crate::error::DevToolsError> for HtmlHotReloadError {
    fn from(e: crate::error::DevToolsError) -> Self {
        Self::Watch(e.to_string())
    }
}

/// HTML 热重载管理器。
///
/// 监控指定的 `.html` 文件，检测到变更时自动重新解析为 WidgetView 树。
/// 与现有 [`FileWatcher`] 集成，复用 debounce 机制。
///
/// # 类型参数
///
/// * `M` - 应用消息类型（传递给 [`parse_html_file`]）。
pub struct HtmlHotReload<M: AppMessage> {
    /// 文件监控器。
    watcher: FileWatcher,
    /// 被监控的 HTML 文件路径（用户传入的原始路径）。
    html_path: PathBuf,
    /// 规范化的 HTML 文件路径（用于与 notify 事件路径比较）。
    canonical_html_path: PathBuf,
    /// 当前 WidgetView 缓存——用于快速返回最新解析结果。
    current_view: WidgetView<M>,
}

impl<M: AppMessage> HtmlHotReload<M> {
    /// 创建新的 HTML 热重载管理器。
    ///
    /// 创建时立即解析一次 HTML 文件以填充初始视图。
    ///
    /// # 参数
    ///
    /// * `config` - 热重载配置（`watch_paths` 需包含 `html_path` 所在目录）
    /// * `html_path` - HTML 文件路径
    ///
    /// # 错误
    ///
    /// * 文件监控创建失败
    /// * HTML 文件读取或解析失败
    pub fn new(
        config: &HotReloadConfig,
        html_path: impl AsRef<Path>,
    ) -> Result<Self, HtmlHotReloadError> {
        let html_path = html_path.as_ref().to_path_buf();
        // 规范化路径以匹配 notify 事件路径（macOS 上 /tmp → /private/tmp）
        let canonical_html_path =
            std::fs::canonicalize(&html_path).unwrap_or_else(|_| html_path.clone());
        let watcher = FileWatcher::new(config)?;
        let current_view = parse_html_file_from_path(&html_path)?;
        Ok(Self {
            watcher,
            html_path,
            canonical_html_path,
            current_view,
        })
    }

    /// 检查文件变更，若有则重新解析。
    ///
    /// 在主循环中调用。debouce 窗口由配置中的 [`HotReloadConfig::debounce_duration`]
    /// 控制（默认 300ms）。
    ///
    /// # 返回值
    ///
    /// * `Ok(Some(view))` - HTML 文件已变更，返回新的 `WidgetView`
    /// * `Ok(None)` - 无变更
    /// * `Err(_)` - 解析或监控错误
    pub fn check_and_reload(&mut self) -> Result<Option<WidgetView<M>>, HtmlHotReloadError> {
        let changes = self.watcher.check_changes();
        let html_changed = changes
            .iter()
            .any(|c| path_matches(&c.path, &self.canonical_html_path));

        if html_changed {
            let new_view = parse_html_file_from_path(&self.html_path)?;
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
    /// 若 HTML 文件有变更则立即重新解析。
    pub fn flush_and_reload(&mut self) -> Result<Option<WidgetView<M>>, HtmlHotReloadError> {
        let changes = self.watcher.flush();
        let html_changed = changes
            .iter()
            .any(|c| path_matches(&c.path, &self.canonical_html_path));

        if html_changed {
            let new_view = parse_html_file_from_path(&self.html_path)?;
            self.current_view = new_view.clone();
            Ok(Some(new_view))
        } else {
            Ok(None)
        }
    }
}

impl<M: AppMessage> std::fmt::Debug for HtmlHotReload<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HtmlHotReload")
            .field("html_path", &self.html_path)
            .field("widget_type", &self.current_view.widget_type)
            .finish_non_exhaustive()
    }
}

/// 从文件路径解析 HTML。包装 [`parse_html_file`] 以处理路径不存在的情况。
fn parse_html_file_from_path<M: AppMessage>(
    path: &Path,
) -> Result<WidgetView<M>, HtmlHotReloadError> {
    if !path.exists() {
        return Err(HtmlHotReloadError::Watch(format!(
            "HTML 文件不存在: {}",
            path.display()
        )));
    }
    Ok(parse_html_file(path)?)
}

/// 比较 notify 事件路径与规范化的 HTML 文件路径。
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

    /// 创建一个包含简单 HTML 文件的临时目录和配置。
    fn setup_test_env(html_content: &str) -> (tempfile::TempDir, PathBuf, HotReloadConfig) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let html_path = dir.path().join("test.html");
        std::fs::write(&html_path, html_content).expect("写入 HTML 文件失败");

        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };

        (dir, html_path, config)
    }

    // === 创建测试 ===

    #[test]
    fn test_new_creates_successfully() {
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Hello" />"#);
        let result = HtmlHotReload::<TestMsg>::new(&config, &html_path);
        assert!(result.is_ok(), "应成功创建 HtmlHotReload");
    }

    #[test]
    fn test_new_preserves_initial_view() {
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Hello" />"#);
        let reloader =
            HtmlHotReload::<TestMsg>::new(&config, &html_path).expect("创建 HtmlHotReload 失败");
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
        let non_existent = dir.path().join("no_such_file.html");
        let result = HtmlHotReload::<TestMsg>::new(&config, &non_existent);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("不存在"), "错误应提及文件不存在: {msg}");
    }

    // === 重载测试 ===

    #[test]
    fn test_check_and_reload_returns_none_without_changes() {
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Hello" />"#);
        let mut reloader =
            HtmlHotReload::<TestMsg>::new(&config, &html_path).expect("创建 HtmlHotReload 失败");

        // 等待 watcher 稳定
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "无变更应返回 None");
    }

    #[test]
    fn test_check_and_reload_detects_change() {
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Hello" />"#);
        let mut reloader =
            HtmlHotReload::<TestMsg>::new(&config, &html_path).expect("创建 HtmlHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件
        std::fs::write(&html_path, r#"<Button label="Click" />"#).expect("写入 HTML 文件失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_ok());
        let maybe_view = result.unwrap();
        assert!(
            maybe_view.is_some(),
            "文件变更应返回新 WidgetView，实际变更列表为空"
        );
        let new_view = maybe_view.unwrap();
        assert_eq!(new_view.widget_type, "Button");
    }

    #[test]
    fn test_check_and_reload_updates_cache() {
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Before" />"#);
        let mut reloader =
            HtmlHotReload::<TestMsg>::new(&config, &html_path).expect("创建 HtmlHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件
        std::fs::write(
            &html_path,
            r#"<Column gap="8"><Label text="After" /></Column>"#,
        )
        .expect("写入 HTML 文件失败");
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
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Original" />"#);
        let mut reloader =
            HtmlHotReload::<TestMsg>::new(&config, &html_path).expect("创建 HtmlHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件，立即 flush（不等 debounce）
        std::fs::write(&html_path, r#"<Label text="Flushed" />"#).expect("写入 HTML 文件失败");
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
    fn test_reload_malformed_html() {
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Valid" />"#);
        let mut reloader =
            HtmlHotReload::<TestMsg>::new(&config, &html_path).expect("创建 HtmlHotReload 失败");

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 写入语法错误的 HTML（未闭合的标签）
        std::fs::write(&html_path, r#"<Column><Label text="Oops""#).expect("写入 HTML 文件失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_err(), "畸形 HTML 应返回错误");
    }

    // === 错误类型测试 ===

    #[test]
    fn test_error_display_parse() {
        let parse_err = HtmlParseError::ParseError {
            line: 2,
            col: 1,
            message: "意外".into(),
        };
        let err = HtmlHotReloadError::from(parse_err);
        let msg = err.to_string();
        assert!(msg.contains("HTML 解析失败"));
        assert!(msg.contains("意外"));
    }

    #[test]
    fn test_error_display_watch() {
        let err = HtmlHotReloadError::Watch("目录不存在".into());
        let msg = err.to_string();
        assert!(msg.contains("HTML 热重载监控失败"));
        assert!(msg.contains("目录不存在"));
    }

    #[test]
    fn test_error_from_html_parse_error() {
        let parse_err = HtmlParseError::ParseError {
            line: 1,
            col: 1,
            message: "空".into(),
        };
        let err: HtmlHotReloadError = parse_err.into();
        assert!(matches!(err, HtmlHotReloadError::Parse(_)));
    }

    // === Debug trait 测试 ===

    #[test]
    fn test_debug_output_contains_fields() {
        let (_dir, html_path, config) = setup_test_env(r#"<Label text="Debug" />"#);
        let reloader =
            HtmlHotReload::<TestMsg>::new(&config, &html_path).expect("创建 HtmlHotReload 失败");

        let debug_str = format!("{reloader:?}");
        assert!(debug_str.contains("HtmlHotReload"));
        assert!(debug_str.contains("widget_type"));
    }
}
