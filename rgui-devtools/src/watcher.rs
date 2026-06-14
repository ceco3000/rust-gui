//! 文件变更监控——notify 封装 + debounce 合并。
//!
//! 设计源自 D7 §2.1：使用 `notify` crate 递归监控项目目录，
//! 合并 300ms 内的多次保存为一次重载，按文件扩展名路由到对应热重载处理器。

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::config::HotReloadConfig;
use crate::error::DevToolsError;

/// 文件变更分类——按扩展名路由到对应热重载层级。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeKind {
    /// 样式文件（`.rgss`）——第 1 层，< 200ms。
    Style,
    /// 结构文件（`.rgui`）——第 2 层，< 1s。
    Structure,
    /// Rust 源文件（`.rs`）——第 3 层，2-5s。
    Rust,
    /// 其他文件类型（资源、配置等），当前不触发重载。
    Other,
}

impl FileChangeKind {
    /// 根据文件扩展名分类（大小写不敏感）。
    #[must_use]
    pub fn from_path(path: &std::path::Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_ascii_lowercase().as_str() {
                "rgss" => Self::Style,
                "rgui" => Self::Structure,
                "rs" => Self::Rust,
                _ => Self::Other,
            })
            .unwrap_or(Self::Other)
    }
}

/// 一次经过 debounce 合并后的文件变更事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangeEvent {
    /// 变更的文件路径。
    pub path: PathBuf,
    /// 文件变更类别。
    pub kind: FileChangeKind,
}

/// 文件变更监控器。
///
/// 封装 `notify::RecommendedWatcher`，提供 debounce 合并和按扩展名分类。
/// 使用独立线程接收文件系统事件，通过 `check_changes()` 轮询合并后的变更列表。
///
/// # 使用示例
///
/// ```ignore
/// let config = HotReloadConfig::default();
/// let mut watcher = FileWatcher::new(&config)?;
///
/// // 在主循环中轮询
/// loop {
///     for change in watcher.check_changes() {
///         match change.kind {
///             FileChangeKind::Style => { /* 触发热重载 */ }
///             _ => {}
///         }
///     }
///     std::thread::sleep(std::time::Duration::from_millis(50));
/// }
/// ```
pub struct FileWatcher {
    /// notify 文件系统监控器（必须保持存活以维持监控，否则 drop 会停止事件接收）。
    #[allow(dead_code)]
    watcher: notify::RecommendedWatcher,
    /// 事件接收端。
    rx: Receiver<notify::Result<Event>>,
    /// 当前 debounce 窗口内积累的变更。
    pending: Vec<FileChangeEvent>,
    /// 上次刷新 pending 的时间。
    last_flush: Instant,
    /// debounce 窗口时长。
    debounce_duration: Duration,
}

impl FileWatcher {
    /// 创建新的文件监控器。
    ///
    /// 根据 `config.watch_paths` 递归监控指定目录。
    ///
    /// # 错误
    ///
    /// - 无法创建文件系统监控器时返回 `DevToolsError::WatchFailed`
    pub fn new(config: &HotReloadConfig) -> Result<Self, DevToolsError> {
        // 使用无界通道：notify 回调不应阻塞，文件系统事件不会爆炸式增长
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let _ = tx.send(res);
        })?;

        for path in &config.watch_paths {
            watcher.watch(path, RecursiveMode::Recursive).map_err(|e| {
                DevToolsError::WatchFailed(format!("监控 {} 失败: {e}", path.display()))
            })?;
        }

        Ok(Self {
            watcher,
            rx,
            pending: Vec::new(),
            last_flush: Instant::now(),
            debounce_duration: config.debounce_duration,
        })
    }

    /// 轮询文件变更事件，返回 debounce 合并后的变更列表。
    ///
    /// - 多次快速保存（300ms 窗口内）合并为一次返回
    /// - 相同文件的重复变更只保留最后一次
    /// - 窗口到期时才刷新，否则返回空列表
    pub fn check_changes(&mut self) -> Vec<FileChangeEvent> {
        // 收集所有待处理事件
        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => self.handle_event(event),
                Ok(Err(_)) => {
                    // notify 事件错误（如路径编码问题），忽略单个事件
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // 检查 debounce 窗口是否到期
        let elapsed = self.last_flush.elapsed();
        if elapsed >= self.debounce_duration && !self.pending.is_empty() {
            self.last_flush = Instant::now();
            std::mem::take(&mut self.pending)
        } else {
            Vec::new()
        }
    }

    /// 处理单个 notify 事件。
    fn handle_event(&mut self, event: Event) {
        // 仅处理文件修改/创建事件，忽略删除和元数据变更
        let is_modify = matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_));

        if !is_modify {
            return;
        }

        for path in &event.paths {
            let kind = FileChangeKind::from_path(path);

            // 相同路径的变更去重：移除旧的，保留新的
            self.pending.retain(|e| e.path != *path);

            self.pending.push(FileChangeEvent {
                path: path.clone(),
                kind,
            });
        }
    }

    /// 强制刷新待处理的变更（立即返回，不论 debounce 窗口）。
    #[must_use]
    pub fn flush(&mut self) -> Vec<FileChangeEvent> {
        // 先收集当前队列中的事件
        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => self.handle_event(event),
                Ok(Err(_)) => {},
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        self.last_flush = Instant::now();
        std::mem::take(&mut self.pending)
    }
}

impl std::fmt::Debug for FileWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileWatcher")
            .field("pending_count", &self.pending.len())
            .field("debounce_duration", &self.debounce_duration)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 创建临时的测试配置，监控 temp 目录。
    fn test_config(dir: &std::path::Path) -> HotReloadConfig {
        HotReloadConfig {
            watch_paths: vec![dir.to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        }
    }

    // === FileChangeKind 分类测试 ===

    #[test]
    fn test_kind_style_from_rgss() {
        let kind = FileChangeKind::from_path(std::path::Path::new("theme.rgss"));
        assert_eq!(kind, FileChangeKind::Style);
    }

    #[test]
    fn test_kind_structure_from_rgui() {
        let kind = FileChangeKind::from_path(std::path::Path::new("main.rgui"));
        assert_eq!(kind, FileChangeKind::Structure);
    }

    #[test]
    fn test_kind_rust_from_rs() {
        let kind = FileChangeKind::from_path(std::path::Path::new("src/main.rs"));
        assert_eq!(kind, FileChangeKind::Rust);
    }

    #[test]
    fn test_kind_other_from_unknown_extension() {
        let kind = FileChangeKind::from_path(std::path::Path::new("image.png"));
        assert_eq!(kind, FileChangeKind::Other);
    }

    #[test]
    fn test_kind_other_no_extension() {
        let kind = FileChangeKind::from_path(std::path::Path::new("Makefile"));
        assert_eq!(kind, FileChangeKind::Other);
    }

    #[test]
    fn test_kind_nested_rgss_path() {
        let kind = FileChangeKind::from_path(std::path::Path::new("styles/dark/theme.rgss"));
        assert_eq!(kind, FileChangeKind::Style);
    }

    // === FileWatcher 创建测试 ===

    #[test]
    fn test_create_file_watcher() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = test_config(dir.path());
        let watcher = FileWatcher::new(&config);
        assert!(watcher.is_ok(), "FileWatcher 应成功创建");
    }

    #[test]
    fn test_create_watcher_multiple_paths() {
        let dir1 = tempfile::tempdir().expect("创建临时目录失败");
        let dir2 = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()],
            debounce_duration: Duration::from_millis(100),
            ..Default::default()
        };
        let watcher = FileWatcher::new(&config);
        assert!(watcher.is_ok(), "多路径 FileWatcher 应成功创建");
    }

    #[test]
    fn test_check_changes_returns_empty_initially() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = test_config(dir.path());
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");
        let changes = watcher.check_changes();
        assert!(changes.is_empty(), "初始应无变更");
    }

    // === 集成测试：实际文件变更检测 ===

    #[test]
    fn test_detect_file_create() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = test_config(dir.path());
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        // 创建文件
        let file_path = dir.path().join("theme.rgss");
        std::fs::write(&file_path, b"/* test */").expect("写入文件失败");

        // 等待文件系统事件传播 + debounce 窗口
        std::thread::sleep(Duration::from_millis(100));

        let changes = watcher.check_changes();
        let rgss_changes: Vec<_> = changes
            .iter()
            .filter(|c| c.kind == FileChangeKind::Style)
            .collect();
        assert!(
            !rgss_changes.is_empty(),
            "应检测到 .rgss 文件创建，实际变更: {changes:?}"
        );
    }

    #[test]
    fn test_detect_file_modify() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = test_config(dir.path());
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        // 先创建文件，等 watcher 稳定
        let file_path = dir.path().join("main.rs");
        std::fs::write(&file_path, b"fn main() {}").expect("写入文件失败");
        std::thread::sleep(Duration::from_millis(100));

        // 消耗掉创建事件
        let _ = watcher.flush();

        // 修改文件
        std::fs::write(&file_path, b"fn main() { println!(\"hi\"); }").expect("写入文件失败");
        std::thread::sleep(Duration::from_millis(100));

        let changes = watcher.check_changes();
        let rs_changes: Vec<_> = changes
            .iter()
            .filter(|c| c.kind == FileChangeKind::Rust)
            .collect();
        assert!(
            !rs_changes.is_empty(),
            "应检测到 .rs 文件修改，实际变更: {changes:?}"
        );
    }

    #[test]
    fn test_debounce_merges_rapid_changes() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(200),
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        // 快速连续写入同一文件
        let file_path = dir.path().join("theme.rgss");
        for i in 0..5 {
            std::fs::write(&file_path, format!("/* version {i} */")).expect("写入文件失败");
            std::thread::sleep(Duration::from_millis(10));
        }

        // 等待 debounce 窗口
        std::thread::sleep(Duration::from_millis(250));

        let changes = watcher.check_changes();
        let rgss_changes: Vec<_> = changes
            .iter()
            .filter(|c| c.kind == FileChangeKind::Style)
            .collect();

        // debounce 合并后，同一文件应只出现一次
        assert_eq!(
            rgss_changes.len(),
            1,
            "同一文件 5 次快速保存应合并为 1 次变更，实际: {rgss_changes:?}"
        );
    }

    #[test]
    fn test_debounce_separates_distinct_changes() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        // 第一批变更
        let file1 = dir.path().join("a.rgss");
        std::fs::write(&file1, b"/* a */").expect("写入文件失败");
        std::thread::sleep(Duration::from_millis(80));

        let batch1 = watcher.check_changes();
        assert!(!batch1.is_empty(), "第一批变更应被检测到");

        // 第二批变更
        let file2 = dir.path().join("b.rgss");
        std::fs::write(&file2, b"/* b */").expect("写入文件失败");
        std::thread::sleep(Duration::from_millis(80));

        let batch2 = watcher.check_changes();
        assert!(!batch2.is_empty(), "第二批变更应被检测到");
    }

    #[test]
    fn test_flush_returns_immediately() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_secs(60), // 极长的 debounce
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        let file_path = dir.path().join("theme.rgss");
        std::fs::write(&file_path, b"/* test */").expect("写入文件失败");
        std::thread::sleep(Duration::from_millis(100));

        // flush 应忽略 debounce，立即返回
        let changes = watcher.flush();
        assert!(
            !changes.is_empty(),
            "flush 应忽略 debounce 立即返回变更，实际: {changes:?}"
        );
    }

    #[test]
    fn test_delete_event_does_not_panic() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = test_config(dir.path());
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        // 创建文件并等待事件传播
        let file_path = dir.path().join("theme.rgss");
        std::fs::write(&file_path, b"/* test */").expect("写入文件失败");
        std::thread::sleep(Duration::from_millis(200));

        // 消耗创建/修改事件
        let _ = watcher.flush();

        // 确认没有残留事件
        std::thread::sleep(Duration::from_millis(100));
        let residual = watcher.check_changes();
        assert!(
            residual.is_empty(),
            "flush 后应无残留事件，实际: {residual:?}"
        );

        // 删除文件
        std::fs::remove_file(&file_path).expect("删除文件失败");
        std::thread::sleep(Duration::from_millis(200));

        // macOS FSEvents 可能在删除时产生 Modify 事件，因此不强制断言为空
        // 只验证 FileWatcher 不会崩溃
        let _changes = watcher.check_changes();
    }

    #[test]
    fn test_file_change_event_equality() {
        let a = FileChangeEvent {
            path: PathBuf::from("a.rgss"),
            kind: FileChangeKind::Style,
        };
        let b = FileChangeEvent {
            path: PathBuf::from("a.rgss"),
            kind: FileChangeKind::Style,
        };
        let c = FileChangeEvent {
            path: PathBuf::from("b.rgss"),
            kind: FileChangeKind::Style,
        };

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_multiple_file_types_in_one_batch() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = test_config(dir.path());
        let mut watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        // 创建不同类型的文件
        std::fs::write(dir.path().join("theme.rgss"), b"/* style */").expect("写入失败");
        std::fs::write(dir.path().join("layout.rgui"), b"<layout/>").expect("写入失败");
        std::fs::write(dir.path().join("main.rs"), b"fn main() {}").expect("写入失败");
        std::thread::sleep(Duration::from_millis(100));

        let changes = watcher.check_changes();
        let kinds: Vec<FileChangeKind> = changes.iter().map(|c| c.kind.clone()).collect();

        assert!(
            kinds.contains(&FileChangeKind::Style),
            "应包含 Style 变更，实际: {kinds:?}"
        );
        assert!(
            kinds.contains(&FileChangeKind::Structure),
            "应包含 Structure 变更，实际: {kinds:?}"
        );
        assert!(
            kinds.contains(&FileChangeKind::Rust),
            "应包含 Rust 变更，实际: {kinds:?}"
        );
    }

    #[test]
    fn test_filewatcher_debug_format() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = test_config(dir.path());
        let watcher = FileWatcher::new(&config).expect("创建 FileWatcher");

        let debug_str = format!("{watcher:?}");
        assert!(
            debug_str.contains("FileWatcher"),
            "Debug 输出应包含类型名，实际: {debug_str}"
        );
        assert!(
            debug_str.contains("pending_count"),
            "Debug 输出应包含 pending_count，实际: {debug_str}"
        );
        assert!(
            debug_str.contains("debounce_duration"),
            "Debug 输出应包含 debounce_duration，实际: {debug_str}"
        );
    }
}
