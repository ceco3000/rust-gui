//! `.rhai` 热重载管理器——监控 `.rhai` 文件，变更时自动重新编译脚本并替换命令处理器。
//!
//! ## 设计
//!
//! `RhaiHotReload` 封装了文件监控（[`FileWatcher`]）和 Rhai 脚本注册
//! （[`CommandRegistry`]），提供简单的轮询接口：在主循环中调用
//! [`check_and_reload`](RhaiHotReload::check_and_reload)，
//! 若 `.rhai` 文件有变更则自动重新编译脚本。
//!
//! ### 脚本更新策略
//!
//! 每次 `.rhai` 文件变更时，调用 `CommandRegistry::register_script()` 重新注册。
//! Rhai AST 合并时新函数定义会屏蔽旧定义，实现热重载。
//!
//! ### 使用示例
//!
//! ```ignore
//! use rgui_devtools::rhai_hot_reload::RhaiHotReload;
//! use rgui_devtools::config::HotReloadConfig;
//!
//! let config = HotReloadConfig::default();
//! let mut reloader = RhaiHotReload::new(&config)?;
//! reloader.watch("scripts/handlers.rhai")?;
//!
//! // 在主循环中
//! loop {
//!     if reloader.check_and_reload()? {
//!         // 脚本已重新加载，CommandRegistry 已更新
//!         let registry = reloader.registry();
//!         // ... 使用更新后的 registry
//!     }
//! }
//! ```
//!
//! 设计源自 D8 RH03、D7 §10。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rgui_script::CommandRegistry;

use crate::config::HotReloadConfig;
use crate::error::DevToolsError;
use crate::watcher::FileWatcher;

/// `.rhai` 热重载错误。
#[derive(Debug)]
#[non_exhaustive]
pub enum RhaiHotReloadError {
    /// 文件监控或 I/O 错误。
    Watch(String),
    /// Rhai 脚本编译或执行失败。
    Script(String),
}

impl std::fmt::Display for RhaiHotReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Watch(msg) => write!(f, ".rhai 热重载监控失败: {msg}"),
            Self::Script(e) => write!(f, ".rhai 脚本编译失败: {e}"),
        }
    }
}

impl std::error::Error for RhaiHotReloadError {}

impl From<DevToolsError> for RhaiHotReloadError {
    fn from(e: DevToolsError) -> Self {
        Self::Watch(e.to_string())
    }
}

impl From<Box<rhai::EvalAltResult>> for RhaiHotReloadError {
    fn from(e: Box<rhai::EvalAltResult>) -> Self {
        Self::Script(e.to_string())
    }
}

/// `.rhai` 热重载管理器。
///
/// 监控指定的 `.rhai` 文件，检测到变更时自动重新编译脚本并更新
/// [`CommandRegistry`]。复用现有 [`FileWatcher`] 的 debounce 机制。
///
/// # 状态保持
///
/// 重新注册脚本时仅更新函数定义，不丢失已有的类型注册和全局状态。
/// 修改 `.rhai` 函数后 < 500ms 生效（debounce 300ms + 编译时间）。
pub struct RhaiHotReload {
    /// 文件监控器。
    watcher: FileWatcher,
    /// Rhai 命令注册表（用于注册和调用脚本函数）。
    registry: CommandRegistry,
    /// 被监控的脚本路径 → 规范化路径映射，用于匹配 notify 事件。
    watched_scripts: HashMap<PathBuf, PathBuf>,
    /// 规范化路径 → 源文件路径（用于重新读取源文件）。
    canonical_to_source: HashMap<PathBuf, PathBuf>,
}

impl RhaiHotReload {
    /// 使用外部提供的 [`CommandRegistry`] 创建热重载管理器（RS04）。
    ///
    /// 与 [`new`](Self::new) 的区别：接受外部创建的 `CommandRegistry`，
    /// 使渲染线程和 Rhai 引擎可以共享同一份 `PropRegistry` 和 `WidgetIdBimap`。
    ///
    /// # 参数
    ///
    /// * `config` - 热重载配置
    /// * `registry` - 外部创建的 `CommandRegistry`（已包含共享状态）
    ///
    /// # 错误
    ///
    /// * 文件监控创建失败
    pub fn with_registry(
        config: &HotReloadConfig,
        registry: CommandRegistry,
    ) -> Result<Self, RhaiHotReloadError> {
        let watcher = FileWatcher::new(config)?;
        Ok(Self {
            watcher,
            registry,
            watched_scripts: HashMap::new(),
            canonical_to_source: HashMap::new(),
        })
    }

    /// 创建新的 `.rhai` 热重载管理器。
    ///
    /// # 参数
    ///
    /// * `config` - 热重载配置（`watch_paths` 需包含 `.rhai` 文件所在目录）
    ///
    /// # 错误
    ///
    /// * 文件监控创建失败
    pub fn new(config: &HotReloadConfig) -> Result<Self, RhaiHotReloadError> {
        Self::with_registry(config, CommandRegistry::new())
    }

    /// 添加一个要监控的 `.rhai` 文件，并立即注册其中的函数。
    ///
    /// # 参数
    ///
    /// * `path` - `.rhai` 文件路径
    ///
    /// # 错误
    ///
    /// * 文件读取或脚本编译失败
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<(), RhaiHotReloadError> {
        let source_path = path.as_ref().to_path_buf();

        // 读取脚本源文件
        let source = std::fs::read_to_string(&source_path).map_err(|e| {
            RhaiHotReloadError::Watch(format!("读取 {} 失败: {e}", source_path.display()))
        })?;

        // 注册脚本
        self.registry.register_script(&source)?;

        // 规范化路径以匹配 notify 事件路径
        let canonical = std::fs::canonicalize(&source_path).unwrap_or_else(|e| {
            log::warn!(target: "rgui::devtools",
                "[rgui] RhaiHotReload: canonicalize({source_path:?}) 失败: {e}，使用原始路径"
            );
            source_path.clone()
        });

        self.watched_scripts
            .insert(source_path.clone(), canonical.clone());
        self.canonical_to_source.insert(canonical, source_path);

        Ok(())
    }

    /// 检查文件变更，若有变更则自动重新编译脚本。
    ///
    /// 在主循环中调用。debounce 窗口由配置中的
    /// [`HotReloadConfig::debounce_duration`] 控制（默认 300ms）。
    ///
    /// # 返回值
    ///
    /// * `Ok(true)` - 至少一个 `.rhai` 文件已变更并重新加载
    /// * `Ok(false)` - 无变更
    /// * `Err(_)` - 编译或监控错误
    pub fn check_and_reload(&mut self) -> Result<bool, RhaiHotReloadError> {
        let changes = self.watcher.check_changes();

        if changes.is_empty() {
            return Ok(false);
        }

        let mut reloaded = false;

        for change in &changes {
            // 检查是否有匹配的规范化路径
            let matched_source =
                self.canonical_to_source
                    .get(&change.path)
                    .cloned()
                    .or_else(|| {
                        // 也尝试直接匹配（非规范化路径）
                        self.canonical_to_source
                            .iter()
                            .find(|(canonical, _)| *canonical == &change.path)
                            .map(|(_, source)| source.clone())
                    });

            if let Some(source_path) = matched_source {
                let source = std::fs::read_to_string(&source_path).map_err(|e| {
                    RhaiHotReloadError::Watch(format!(
                        "重新读取 {} 失败: {e}",
                        source_path.display()
                    ))
                })?;

                self.registry.register_script(&source)?;
                reloaded = true;
            }
        }

        Ok(reloaded)
    }

    /// 获取命令注册表的引用。
    ///
    /// 返回的 `CommandRegistry` 是克隆的，但与内部注册表共享底层
    /// Rhai 引擎和编译后的 AST（通过 `Arc<Mutex<...>>`）。
    #[must_use]
    pub fn registry(&self) -> CommandRegistry {
        self.registry.clone()
    }

    /// 强制重新加载所有被监控的脚本（忽略 debounce 窗口）。
    pub fn flush_and_reload(&mut self) -> Result<bool, RhaiHotReloadError> {
        let _changes = self.watcher.flush();

        let mut reloaded = false;
        let source_paths: Vec<PathBuf> = self.watched_scripts.keys().cloned().collect();

        for source_path in source_paths {
            let source = std::fs::read_to_string(&source_path).map_err(|e| {
                RhaiHotReloadError::Watch(format!("读取 {} 失败: {e}", source_path.display()))
            })?;

            self.registry.register_script(&source)?;
            reloaded = true;
        }

        Ok(reloaded)
    }
}

impl std::fmt::Debug for RhaiHotReload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RhaiHotReload")
            .field("watched_count", &self.watched_scripts.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 创建包含 `.rhai` 脚本文件的临时目录和配置。
    fn setup_test_env(rhai_content: &str) -> (tempfile::TempDir, PathBuf, HotReloadConfig) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let rhai_path = dir.path().join("handlers.rhai");
        std::fs::write(&rhai_path, rhai_content).expect("写入 .rhai 文件失败");

        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };

        (dir, rhai_path, config)
    }

    /// 帮助函数：从 registry 中调用函数并获取 i64 返回值。
    fn call_fn_i64(registry: &mut CommandRegistry, name: &str) -> i64 {
        registry.call_fn::<i64>(name, ()).expect("call_fn 应成功")
    }

    // === 创建测试 ===

    #[test]
    fn test_new_creates_empty_reloader() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };
        let result = RhaiHotReload::new(&config);
        assert!(result.is_ok(), "应成功创建 RhaiHotReload");
    }

    #[test]
    fn test_watch_registers_script_function() {
        let (_dir, rhai_path, config) = setup_test_env("fn get_value() { 42 }");
        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");

        reloader.watch(&rhai_path).expect("watch 应成功");

        let mut registry = reloader.registry();
        let result: i64 = registry.call_fn("get_value", ()).expect("应能调用函数");
        assert_eq!(result, 42);
    }

    #[test]
    fn test_watch_multiple_files() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };

        let path1 = dir.path().join("a.rhai");
        let path2 = dir.path().join("b.rhai");
        std::fs::write(&path1, "fn first() { 1 }").expect("写入失败");
        std::fs::write(&path2, "fn second() { 2 }").expect("写入失败");

        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");
        reloader.watch(&path1).expect("watch 1 失败");
        reloader.watch(&path2).expect("watch 2 失败");

        let mut registry = reloader.registry();
        assert_eq!(call_fn_i64(&mut registry, "first"), 1);
        assert_eq!(call_fn_i64(&mut registry, "second"), 2);
    }

    #[test]
    fn test_watch_invalid_rhai_returns_error() {
        let (_dir, rhai_path, config) = setup_test_env("fn broken( {");
        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");

        let result = reloader.watch(&rhai_path);
        assert!(result.is_err(), "无效脚本应返回错误");
    }

    // === 热重载测试 ===

    #[test]
    fn test_check_and_reload_returns_false_without_changes() {
        let (_dir, rhai_path, config) = setup_test_env("fn get_value() { 42 }");
        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");
        reloader.watch(&rhai_path).expect("watch 失败");

        // 等待 watcher 稳定
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_ok());
        assert!(!result.unwrap(), "无变更应返回 false");
    }

    #[test]
    fn test_check_and_reload_detects_change() {
        let (_dir, rhai_path, config) = setup_test_env("fn get_value() { 1 }");
        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");
        reloader.watch(&rhai_path).expect("watch 失败");

        // 验证初始值
        {
            let mut registry = reloader.registry();
            assert_eq!(call_fn_i64(&mut registry, "get_value"), 1);
        }

        // 等待 watcher 稳定后消耗初始事件
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件
        std::fs::write(&rhai_path, "fn get_value() { 99 }").expect("写入失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_ok());
        assert!(result.unwrap(), "文件变更应触发重载");

        // 验证函数已更新
        let mut registry = reloader.registry();
        assert_eq!(call_fn_i64(&mut registry, "get_value"), 99);
    }

    #[test]
    fn test_check_and_reload_preserves_other_functions() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };

        let path1 = dir.path().join("a.rhai");
        let path2 = dir.path().join("b.rhai");
        std::fs::write(&path1, "fn first() { 1 }").expect("写入失败");
        std::fs::write(&path2, "fn second() { 2 }").expect("写入失败");

        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");
        reloader.watch(&path1).expect("watch 1 失败");
        reloader.watch(&path2).expect("watch 2 失败");

        // 等待稳定
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改 path2
        std::fs::write(&path2, "fn second() { 200 }").expect("写入失败");
        std::thread::sleep(Duration::from_millis(200));

        reloader.check_and_reload().expect("重载应成功");

        let mut registry = reloader.registry();
        // first 不变
        assert_eq!(call_fn_i64(&mut registry, "first"), 1);
        // second 已更新
        assert_eq!(call_fn_i64(&mut registry, "second"), 200);
    }

    #[test]
    fn test_check_and_reload_malformed_rhai_returns_error() {
        let (_dir, rhai_path, config) = setup_test_env("fn get_value() { 42 }");
        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");
        reloader.watch(&rhai_path).expect("watch 失败");

        // 等待稳定
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 写入语法错误的脚本
        std::fs::write(&rhai_path, "fn broken( {").expect("写入失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.check_and_reload();
        assert!(result.is_err(), "畸形脚本应返回错误");
    }

    #[test]
    fn test_flush_and_reload_ignores_debounce() {
        let (_dir, rhai_path, config) = setup_test_env("fn get_value() { 1 }");
        let mut reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");
        reloader.watch(&rhai_path).expect("watch 失败");

        // 等待稳定
        std::thread::sleep(Duration::from_millis(200));
        let _ = reloader.flush_and_reload();

        // 修改文件，立即 flush
        std::fs::write(&rhai_path, "fn get_value() { 999 }").expect("写入失败");
        std::thread::sleep(Duration::from_millis(200));

        let result = reloader.flush_and_reload();
        assert!(result.is_ok());
        assert!(result.unwrap(), "flush 应跳过 debounce");

        let mut registry = reloader.registry();
        assert_eq!(call_fn_i64(&mut registry, "get_value"), 999);
    }

    // === 错误类型测试 ===

    #[test]
    fn test_error_display_watch() {
        let err = RhaiHotReloadError::Watch("目录不存在".into());
        let msg = err.to_string();
        assert!(msg.contains(".rhai 热重载监控失败"));
        assert!(msg.contains("目录不存在"));
    }

    #[test]
    fn test_error_display_script() {
        let err = RhaiHotReloadError::Script("语法错误".into());
        let msg = err.to_string();
        assert!(msg.contains(".rhai 脚本编译失败"));
        assert!(msg.contains("语法错误"));
    }

    // === Debug trait 测试 ===

    #[test]
    fn test_debug_output() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let config = HotReloadConfig {
            watch_paths: vec![dir.path().to_path_buf()],
            debounce_duration: Duration::from_millis(50),
            ..Default::default()
        };
        let reloader = RhaiHotReload::new(&config).expect("创建 RhaiHotReload");
        let debug_str = format!("{reloader:?}");
        assert!(debug_str.contains("RhaiHotReload"));
        assert!(debug_str.contains("watched_count"));
    }
}
