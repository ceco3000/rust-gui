//! 样式热重载——监控 `.rgss` 文件变更并自动重解析。
//!
//! 定义源自 D4 §7。使用 `notify` crate 实现文件系统监控。
//!
//! # 降级策略（D4 §7）
//!
//! - 语法错误时保持旧样式生效，不崩溃。
//! - 在 DevTools 面板中显示错误信息（通过 [`StyleChange::ParseError`] 传递）。
//!
//! # 示例
//!
//! ```ignore
//! use rgui_style::hot_reload::StyleHotReload;
//!
//! let mut reloader = StyleHotReload::new("styles/".into())?;
//! // 在帧循环中定期轮询
//! let changes = reloader.poll_events();
//! for change in &changes {
//!     match change {
//!         StyleChange::Modified { path, .. } => {
//!             log::info!("样式已更新: {}", path.display());
//!         }
//!         _ => {}
//!     }
//! }
//! ```

use crate::parser::parse_rgss;
use crate::selector::StyleRule;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rgui_core::id::WidgetId;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ============================================================================
// StyleChange
// ============================================================================

/// 样式变更通知。
///
/// 当文件系统检测到 `.rgss` 文件变更时，生成此通知。
/// 调用方可根据通知类型决定是否标记受影响的 widget dirty。
#[derive(Debug, Clone, PartialEq)]
pub enum StyleChange {
    /// 样式文件被修改（已存在文件内容变更）。
    Modified {
        /// 变更的文件路径。
        path: PathBuf,
        /// 新样式文件中的规则数量。
        rule_count: usize,
    },
    /// 新的样式文件被添加。
    Added {
        /// 新添加的文件路径。
        path: PathBuf,
        /// 新样式文件中的规则数量。
        rule_count: usize,
    },
    /// 样式文件被删除。
    Removed {
        /// 被删除的文件路径。
        path: PathBuf,
    },
    /// 样式文件解析失败（旧样式保留，不触发崩溃）。
    ParseError {
        /// 解析失败的文件路径。
        path: PathBuf,
        /// 错误描述信息。
        error: String,
    },
}

// ============================================================================
// StyleSheet
// ============================================================================

/// 已解析的样式表——对应一个 `.rgss` 文件。
#[derive(Debug, Clone)]
pub struct StyleSheet {
    /// 该文件中的样式规则。
    pub rules: Vec<StyleRule>,
    /// 源文件路径。
    pub source_path: PathBuf,
}

// ============================================================================
// HotReloadError
// ============================================================================

/// 样式热重载错误类型。
#[derive(Debug, thiserror::Error)]
pub enum HotReloadError {
    /// notify 文件系统监控初始化失败。
    #[error("文件系统监控错误: {0}")]
    Watch(#[from] notify::Error),
    /// IO 错误（文件读写失败）。
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    /// 样式解析错误。
    #[error("样式解析错误: {0}")]
    Parse(#[from] crate::parser::ParseError),
    /// 样式目录不存在。
    #[error("样式目录不存在: {0}")]
    DirectoryNotFound(PathBuf),
}

// ============================================================================
// StyleHotReload
// ============================================================================

/// 样式热重载器。
///
/// 监控指定目录中的 `.rgss` 文件变更，自动重新解析并返回变更通知。
///
/// # 工作流程（D4 §7）
///
/// 1. 构造时开始监控 `style_dir` 目录
/// 2. 在帧循环中调用 `poll_events()` 获取待处理变更
/// 3. 根据变更通知标记受影响的 widget dirty
/// 4. 下一帧自动应用新样式（< 200ms 目标）
///
/// # 消抖
///
/// 300ms 窗口内的多次文件保存操作会被合并为一次处理，避免编辑器保存时的重复触发。
///
/// # 降级策略
///
/// 语法错误时保持旧样式生效并返回 `StyleChange::ParseError`，不触发崩溃。
pub struct StyleHotReload {
    /// notify watcher 实例。`None` 仅在测试模式中。
    watcher: Option<RecommendedWatcher>,
    /// 文件系统事件接收器。
    event_rx: mpsc::Receiver<Result<Event, notify::Error>>,
    /// 已缓存的解析后样式表（路径 → StyleSheet）。
    stylesheets: FxHashMap<PathBuf, StyleSheet>,
    /// 选择器字符串 → 受影响 WidgetId 的反向索引。
    selector_index: FxHashMap<String, Vec<WidgetId>>,
    /// 消抖时间窗口（默认 300ms）。
    debounce: Duration,
    /// 上次事件时间。
    last_event_time: Option<Instant>,
    /// 待处理的文件路径（消抖窗口内累积）。
    pending_paths: Vec<PathBuf>,
}

impl StyleHotReload {
    /// 创建一个新的 [`StyleHotReload`]，开始监控 `style_dir` 目录。
    ///
    /// 自动加载目录中已有的 `.rgss` 文件到缓存。
    ///
    /// # 参数
    ///
    /// * `style_dir` — 样式文件目录路径，必须存在且为目录。
    ///
    /// # 错误
    ///
    /// - `style_dir` 不存在 → [`HotReloadError::DirectoryNotFound`]
    /// - 无法创建文件系统监控器 → [`HotReloadError::Watch`]
    pub fn new(style_dir: PathBuf) -> Result<Self, HotReloadError> {
        if !style_dir.is_dir() {
            return Err(HotReloadError::DirectoryNotFound(style_dir));
        }

        // `std::sync::mpsc::Sender<Result<Event>>` 实现了 `EventHandler` trait。
        let (tx, event_rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx).map_err(HotReloadError::Watch)?;

        let mut slf = Self {
            watcher: Some(watcher),
            event_rx,
            stylesheets: FxHashMap::default(),
            selector_index: FxHashMap::default(),
            debounce: Duration::from_millis(300),
            last_event_time: None,
            pending_paths: Vec::new(),
        };

        // 开始监控目录
        if let Some(w) = &mut slf.watcher {
            w.watch(&style_dir, RecursiveMode::Recursive)
                .map_err(HotReloadError::Watch)?;
        }

        // 加载目录中已有的 .rgss 文件
        slf.load_existing(&style_dir);

        Ok(slf)
    }

    /// 创建一个不连接文件系统监控的 [`StyleHotReload`]（用于测试）。
    #[cfg(test)]
    fn new_test() -> Self {
        let (_tx, event_rx) = mpsc::channel();
        Self {
            watcher: None,
            event_rx,
            stylesheets: FxHashMap::default(),
            selector_index: FxHashMap::default(),
            debounce: Duration::from_millis(300),
            last_event_time: None,
            pending_paths: Vec::new(),
        }
    }

    /// 递归加载目录及其子目录中已有的 `.rgss` 文件。
    fn load_existing(&mut self, dir: &Path) {
        self.load_existing_recursive(dir);
    }

    /// 递归加载的辅助方法。
    fn load_existing_recursive(&mut self, dir: &Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                log::warn!("无法读取样式目录 {}: {e}", dir.display());
                return;
            },
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.load_existing_recursive(&path);
            } else if path.extension().is_some_and(|ext| ext == "rgss") {
                match std::fs::read_to_string(&path) {
                    Ok(source) => match parse_rgss(&source) {
                        Ok(rules) => {
                            let rule_count = rules.len();
                            self.stylesheets.insert(
                                path.clone(),
                                StyleSheet {
                                    rules,
                                    source_path: path.clone(),
                                },
                            );
                            log::info!(
                                "已加载样式文件: {} ({} 条规则)",
                                path.display(),
                                rule_count
                            );
                        },
                        Err(e) => {
                            log::warn!("样式文件解析失败，跳过: {} — {e}", path.display());
                        },
                    },
                    Err(e) => {
                        log::warn!("无法读取样式文件: {} — {e}", path.display());
                    },
                }
            }
        }
    }

    /// 处理一个文件系统事件并返回对应的样式变更通知。
    ///
    /// 处理事件中的所有 `.rgss` 路径，返回最后一个变更通知。
    /// 在大多数 notify 事件中仅包含一个路径，因此单返回值是足够的。
    ///
    /// # 降级行为
    ///
    /// 如果 `.rgss` 文件解析失败，返回 [`StyleChange::ParseError`]
    /// 并保留旧的样式表，不触发崩溃。
    ///
    /// # 参数
    ///
    /// * `event` — 来自 `notify` 的文件系统事件。
    pub fn handle_event(&mut self, event: Event) -> Option<StyleChange> {
        // 跳过非内容变更事件（如访问事件）
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {},
            _ => return None,
        }

        let mut result = None;
        for path in &event.paths {
            if path.extension().is_some_and(|ext| ext == "rgss") {
                match event.kind {
                    EventKind::Remove(_) => {
                        if self.stylesheets.remove(path).is_some() {
                            log::info!("样式文件已删除: {}", path.display());
                            result = Some(StyleChange::Removed { path: path.clone() });
                        }
                    },
                    _ => {
                        // Modify 或 Create：重新读取文件
                        match std::fs::read_to_string(path) {
                            Ok(source) => {
                                result = self.reload_file(path, &source);
                            },
                            Err(e) => {
                                log::warn!("无法读取样式文件: {} — {e}", path.display());
                            },
                        }
                    },
                }
            }
        }
        result
    }

    /// 重新加载指定路径的 `.rgss` 文件。
    ///
    /// 解析 `source` 中的样式规则，缓存结果，并返回相应的变更通知。
    ///
    /// # 返回值
    ///
    /// - `Some(StyleChange::Modified)` — 文件之前已加载，内容已更新
    /// - `Some(StyleChange::Added)` — 文件是新加载的
    /// - `Some(StyleChange::ParseError)` — 解析失败（旧样式保留）
    /// - `None` — 不会发生（当前实现始终返回 `Some`）
    pub fn reload_file(&mut self, path: &Path, source: &str) -> Option<StyleChange> {
        match parse_rgss(source) {
            Ok(rules) => {
                let was_present = self.stylesheets.contains_key(path);
                let rule_count = rules.len();
                self.stylesheets.insert(
                    path.to_path_buf(),
                    StyleSheet {
                        rules,
                        source_path: path.to_path_buf(),
                    },
                );

                if was_present {
                    log::info!("样式文件已重载: {} ({} 条规则)", path.display(), rule_count);
                    Some(StyleChange::Modified {
                        path: path.to_path_buf(),
                        rule_count,
                    })
                } else {
                    log::info!(
                        "新样式文件已加载: {} ({} 条规则)",
                        path.display(),
                        rule_count
                    );
                    Some(StyleChange::Added {
                        path: path.to_path_buf(),
                        rule_count,
                    })
                }
            },
            Err(e) => {
                // 降级策略（D4 §7）：解析失败时保持旧样式
                log::warn!("样式文件解析失败，保留旧样式: {} — {e}", path.display());
                Some(StyleChange::ParseError {
                    path: path.to_path_buf(),
                    error: e.to_string(),
                })
            },
        }
    }

    /// 非阻塞轮询待处理的文件系统事件（带 300ms 消抖）。
    ///
    /// 在消抖窗口内的多次保存操作会被合并为一次处理。
    /// 应在每帧循环或定时器中调用。
    pub fn poll_events(&mut self) -> Vec<StyleChange> {
        // 收集所有待处理事件中的 .rgss 路径
        for res in self.event_rx.try_iter() {
            match res {
                Ok(event) => {
                    let is_relevant = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    );
                    if is_relevant {
                        for path in &event.paths {
                            if path.extension().is_some_and(|ext| ext == "rgss")
                                && !self.pending_paths.contains(path)
                            {
                                self.pending_paths.push(path.clone());
                            }
                        }
                        self.last_event_time = Some(Instant::now());
                    }
                },
                Err(e) => {
                    log::warn!("文件系统监控错误: {e}");
                },
            }
        }

        // 消抖检查：仍在消抖窗口内时不处理
        if self.pending_paths.is_empty() {
            return Vec::new();
        }

        if let Some(last) = self.last_event_time {
            if last.elapsed() < self.debounce {
                return Vec::new();
            }
        }

        // 处理所有待处理路径
        let paths = std::mem::take(&mut self.pending_paths);
        let mut changes = Vec::new();

        for path in paths {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(source) => {
                        if let Some(change) = self.reload_file(&path, &source) {
                            changes.push(change);
                        }
                    },
                    Err(e) => {
                        log::warn!("无法读取样式文件: {} — {e}", path.display());
                    },
                }
            } else if self.stylesheets.remove(&path).is_some() {
                // 文件已被删除
                changes.push(StyleChange::Removed { path: path.clone() });
            }
        }

        changes
    }

    /// 注册选择器到 WidgetId 的反向映射。
    ///
    /// 当对应选择器的样式变更时，可以通过 [`Self::affected_widgets`] 查询受影响的 widget。
    /// 同一 WidgetId 重复注册会被自动去重。
    ///
    /// # 参数
    ///
    /// * `selector` — 选择器文本（如 `"Button"`、`".primary"`）
    /// * `widget_id` — 受该选择器影响的 Widget
    pub fn register_widget(&mut self, selector: impl Into<String>, widget_id: WidgetId) {
        let ids = self.selector_index.entry(selector.into()).or_default();
        if !ids.contains(&widget_id) {
            ids.push(widget_id);
        }
    }

    /// 取消注册选择器到 WidgetId 的映射。
    ///
    /// Widget 销毁时调用此方法清理索引中的死引用。
    ///
    /// # 参数
    ///
    /// * `selector` — 选择器文本
    /// * `widget_id` — 需要移除的 Widget
    pub fn unregister_widget(&mut self, selector: &str, widget_id: WidgetId) {
        if let Some(ids) = self.selector_index.get_mut(selector) {
            ids.retain(|id| *id != widget_id);
            if ids.is_empty() {
                self.selector_index.remove(selector);
            }
        }
    }

    /// 获取指定选择器影响的 WidgetId 列表。
    #[must_use]
    pub fn affected_widgets(&self, selector: &str) -> &[WidgetId] {
        self.selector_index.get(selector).map_or(&[], Vec::as_slice)
    }

    /// 获取所有已缓存的样式表。
    #[must_use]
    pub fn stylesheets(&self) -> &FxHashMap<PathBuf, StyleSheet> {
        &self.stylesheets
    }

    /// 获取已缓存样式表的数量。
    #[must_use]
    pub fn stylesheet_count(&self) -> usize {
        self.stylesheets.len()
    }
}

impl std::fmt::Debug for StyleHotReload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StyleHotReload")
            .field(
                "watcher",
                if self.watcher.is_some() {
                    &"Some"
                } else {
                    &"None"
                },
            )
            .field("stylesheets", &self.stylesheets.len())
            .field("selector_index", &self.selector_index.len())
            .field("debounce", &self.debounce)
            .field("pending_paths", &self.pending_paths.len())
            .finish()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::EventAttributes;

    #[test]
    fn new_test_creates_empty_reloader() {
        let mut reloader = StyleHotReload::new_test();
        assert_eq!(reloader.stylesheet_count(), 0);
        assert!(reloader.watcher.is_none());

        // poll_events 对空 reloader 应返回空
        let changes = reloader.poll_events();
        assert!(changes.is_empty());
    }

    #[test]
    fn reload_file_adds_and_modifies() {
        let mut reloader = StyleHotReload::new_test();
        let path = PathBuf::from("test.rgss");
        let source = "Button { color: red; }".to_string();

        // 首次加载 → Added
        let change = reloader.reload_file(&path, &source);
        assert!(change.is_some());
        assert_eq!(reloader.stylesheet_count(), 1);

        match change.unwrap() {
            StyleChange::Added { rule_count, .. } => assert_eq!(rule_count, 1),
            other => panic!("期望 Added，得到 {other:?}"),
        }

        // 再次加载 → Modified
        let change = reloader.reload_file(&path, &source);
        assert!(change.is_some());
        match change.unwrap() {
            StyleChange::Modified { rule_count, .. } => assert_eq!(rule_count, 1),
            other => panic!("期望 Modified，得到 {other:?}"),
        }
    }

    #[test]
    fn reload_file_parse_error_retains_old_style() {
        let mut reloader = StyleHotReload::new_test();
        let path = PathBuf::from("test.rgss");

        // 先加载有效样式
        reloader.reload_file(&path, "Button { color: red; }");
        assert_eq!(reloader.stylesheet_count(), 1);

        // 加载无效语法 → ParseError（`@` 不是合法标识符起始），旧样式保留
        let change = reloader.reload_file(&path, "Button { @x: 1; }");
        assert!(change.is_some());
        match change.unwrap() {
            StyleChange::ParseError { .. } => {}, // 符合预期
            other => panic!("期望 ParseError，得到 {other:?}"),
        }
        // 旧样式应保留（前一次成功加载的内容）
        assert_eq!(reloader.stylesheet_count(), 1);
    }

    #[test]
    fn handle_event_modify_reloads_file() {
        let mut reloader = StyleHotReload::new_test();

        // 使用临时目录中的唯一路径避免并行测试冲突
        let tmp = std::env::temp_dir().join("rgui-style-test-modify.rgss");
        let path = tmp.clone();

        // 先写入文件
        std::fs::write(&path, "Button { color: red; }").unwrap();

        // 模拟 Modify 事件
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            paths: vec![path.clone()],
            attrs: EventAttributes::new(),
        };

        let change = reloader.handle_event(event);
        assert!(change.is_some());
        match change.unwrap() {
            StyleChange::Added { .. } => {}, // 文件应被加载为 Added
            other => panic!("期望 Added，得到 {other:?}"),
        }

        // 清理
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn handle_event_remove_removes_stylesheet() {
        let mut reloader = StyleHotReload::new_test();
        let path = PathBuf::from("test.rgss");

        // 先加载
        reloader.reload_file(&path, "Button { color: red; }");
        assert_eq!(reloader.stylesheet_count(), 1);

        // 模拟 Remove 事件
        let event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![path.clone()],
            attrs: EventAttributes::new(),
        };

        let change = reloader.handle_event(event);
        assert!(change.is_some());
        match change.unwrap() {
            StyleChange::Removed { .. } => {},
            other => panic!("期望 Removed，得到 {other:?}"),
        }
        assert_eq!(reloader.stylesheet_count(), 0);
    }

    #[test]
    fn register_and_query_affected_widgets() {
        let mut reloader = StyleHotReload::new_test();
        let id1 = WidgetId::from_u64(1);
        let id2 = WidgetId::from_u64(2);

        reloader.register_widget("Button", id1);
        reloader.register_widget("Button", id2);
        reloader.register_widget(".primary", id1);

        assert_eq!(reloader.affected_widgets("Button"), &[id1, id2]);
        assert_eq!(reloader.affected_widgets(".primary"), &[id1]);
        assert!(reloader.affected_widgets("Nonexistent").is_empty());
    }

    #[test]
    fn register_widget_dedups_duplicate_ids() {
        let mut reloader = StyleHotReload::new_test();
        let id = WidgetId::from_u64(42);

        // 注册两次同一个 ID
        reloader.register_widget("Button", id);
        reloader.register_widget("Button", id);

        assert_eq!(reloader.affected_widgets("Button").len(), 1);
        assert_eq!(reloader.affected_widgets("Button"), &[id]);
    }

    #[test]
    fn unregister_widget_removes_id() {
        let mut reloader = StyleHotReload::new_test();
        let id1 = WidgetId::from_u64(1);
        let id2 = WidgetId::from_u64(2);

        reloader.register_widget("Button", id1);
        reloader.register_widget("Button", id2);
        assert_eq!(reloader.affected_widgets("Button").len(), 2);

        // 取消注册 id1
        reloader.unregister_widget("Button", id1);
        assert_eq!(reloader.affected_widgets("Button"), &[id2]);

        // 取消注册 id2 → selector 条目被自动清理
        reloader.unregister_widget("Button", id2);
        assert!(reloader.affected_widgets("Button").is_empty());
    }

    #[test]
    fn unregister_nonexistent_selector_is_noop() {
        let mut reloader = StyleHotReload::new_test();
        let id = WidgetId::from_u64(1);
        reloader.unregister_widget("Nonexistent", id); // 不应 panic
    }

    #[test]
    fn handle_non_rgss_event_is_ignored() {
        let mut reloader = StyleHotReload::new_test();

        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            paths: vec![PathBuf::from("styles.css")], // 不是 .rgss
            attrs: EventAttributes::new(),
        };

        let change = reloader.handle_event(event);
        assert!(change.is_none());
    }

    #[test]
    fn handle_access_event_is_ignored() {
        let mut reloader = StyleHotReload::new_test();

        let event = Event {
            kind: EventKind::Access(notify::event::AccessKind::Read),
            paths: vec![PathBuf::from("test.rgss")],
            attrs: EventAttributes::new(),
        };

        let change = reloader.handle_event(event);
        assert!(change.is_none());
    }

    #[test]
    fn handle_create_event_processes_file() {
        let mut reloader = StyleHotReload::new_test();

        // 使用临时目录中的唯一路径避免并行测试冲突
        let tmp = std::env::temp_dir().join("rgui-style-test-create.rgss");
        let path = tmp.clone();

        // 先写入文件
        std::fs::write(&path, "Label { font-size: 12px; }").unwrap();

        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![path.clone()],
            attrs: EventAttributes::new(),
        };

        let change = reloader.handle_event(event);
        assert!(change.is_some());
        match change.unwrap() {
            StyleChange::Added { rule_count, .. } => assert_eq!(rule_count, 1),
            other => panic!("期望 Added，得到 {other:?}"),
        }

        // 清理
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn stylesheet_count_tracking() {
        let mut reloader = StyleHotReload::new_test();
        assert_eq!(reloader.stylesheet_count(), 0);

        reloader.reload_file(&PathBuf::from("a.rgss"), "Button { x: 1; }");
        assert_eq!(reloader.stylesheet_count(), 1);

        reloader.reload_file(&PathBuf::from("b.rgss"), ".cls { y: 2; }");
        assert_eq!(reloader.stylesheet_count(), 2);

        // 同名文件替换
        reloader.reload_file(&PathBuf::from("a.rgss"), "Button { x: 3; }");
        assert_eq!(reloader.stylesheet_count(), 2);
    }

    #[test]
    fn reload_file_tracks_rules_from_multi_rule_source() {
        let mut reloader = StyleHotReload::new_test();
        let path = PathBuf::from("test.rgss");
        let source = "\
            Button { color: red; }\n\
            .primary { font-size: 14px; }\n\
            Label { color: blue; }";

        let change = reloader.reload_file(&path, source);
        match change.unwrap() {
            StyleChange::Added { rule_count, .. } => assert_eq!(rule_count, 3),
            other => panic!("期望 Added(3)，得到 {other:?}"),
        }
    }
}
