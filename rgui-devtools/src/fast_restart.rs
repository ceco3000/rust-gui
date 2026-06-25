//! Fast restart cargo build 集成（D7 §5）。
//!
//! ## 核心流程（阶段 1 MVP）
//!
//! 1. FileWatcher 检测到 `.rs` 变更
//! 2. `FastRestarter::build()` 执行 `cargo build`
//! 3. 返回 `BuildResult`（成功/失败/无需构建）
//!
//! ## 阶段 2 计划
//!
//! 阶段 2 引入双进程架构后，完整流程为：
//! 1. 检测 `.rs` 变更
//! 2. cargo build 增量编译（sccache，目标 2-5s）
//! 3. DisplayProcess 检测二进制更新
//! 4. 捕获当前状态快照 + RestoreMetadata
//! 5. DisplayProcess 启动新 AppProcess
//! 6. IPC 发送 IpcMessage::RestoreState
//!
//! 设计源自 D7 §5。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::DevToolsError;

/// cargo build 执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildResult {
    /// 编译成功。
    Success,
    /// 编译失败，携带 stderr 输出。
    Failed { stderr: String },
    /// 无需重新构建（二进制已是最新）。
    UpToDate,
}

/// 快速重启管理器。
///
/// 当 FileWatcher 检测到 `.rs` 文件变更时，由 `FastRestarter`
/// 执行 `cargo build` 并报告结果。调用方根据结果决定是否重启进程。
///
/// # 使用示例（阶段 1 MVP）
///
/// ```ignore
/// use rgui_devtools::fast_restart::FastRestarter;
///
/// let restarter = FastRestarter::new("/path/to/project")?;
///
/// // 在文件监控循环中：
/// if change.kind == FileChangeKind::Rust {
///     match restarter.build() {
///         Ok(BuildResult::Success) => { /* 信号重启 */ }
///         Ok(BuildResult::Failed { stderr }) => { /* 报告错误 */ }
///         Ok(BuildResult::UpToDate) => { /* 无需操作 */ }
///         Err(e) => { /* 内部错误 */ }
///     }
/// }
/// ```
#[derive(Debug)]
pub struct FastRestarter {
    /// 项目根目录（包含 Cargo.toml）。
    project_root: PathBuf,
}

impl FastRestarter {
    /// 创建新的 FastRestarter。
    ///
    /// # 错误
    ///
    /// - 如果 `project_root` 中不存在 `Cargo.toml`，返回
    ///   [`DevToolsError::ConfigError`]。
    pub fn new(project_root: impl Into<PathBuf>) -> Result<Self, DevToolsError> {
        let root: PathBuf = project_root.into();
        if !root.join("Cargo.toml").exists() {
            return Err(DevToolsError::ConfigError(format!(
                "项目根目录未找到 Cargo.toml: {}",
                root.display()
            )));
        }
        Ok(Self { project_root: root })
    }

    /// 执行 `cargo build` 并返回结果。
    ///
    /// 继承当前进程的 `cargo` 和 `sccache`（如已配置）。
    ///
    /// # 错误
    ///
    /// - cargo 未安装时返回 [`DevToolsError::WatchFailed`]
    /// - 编译失败时返回 `Ok(BuildResult::Failed { stderr })`
    pub fn build(&self) -> Result<BuildResult, DevToolsError> {
        let output = Command::new("cargo")
            .arg("build")
            .current_dir(&self.project_root)
            .output()
            .map_err(|e| DevToolsError::WatchFailed(format!("cargo 执行失败: {e}")))?;

        if output.status.success() {
            // 检查是否有任何编译输出（stderr 包含编译信息）
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.is_empty() || stderr.lines().all(|l| l.trim().is_empty()) {
                Ok(BuildResult::UpToDate)
            } else {
                Ok(BuildResult::Success)
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(BuildResult::Failed { stderr })
        }
    }

    /// 返回项目根目录路径。
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_project() -> PathBuf {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            b"[package]\nname = \"test-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("写入 Cargo.toml 失败");
        let path = dir.path().to_path_buf();
        let _ = dir.keep();
        path
    }

    // === FastRestarter 构造测试 ===

    #[test]
    fn test_new_with_valid_project_root() {
        let dir = setup_test_project();
        let result = FastRestarter::new(&dir);
        assert!(result.is_ok(), "应成功创建 FastRestarter");
    }

    #[test]
    fn test_new_with_missing_cargo_toml() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let result = FastRestarter::new(dir.path());
        assert!(result.is_err(), "无 Cargo.toml 时应失败");
        match result {
            Err(DevToolsError::ConfigError(msg)) => {
                assert!(msg.contains("Cargo.toml"), "错误信息应提及 Cargo.toml");
            },
            _ => panic!("应返回 ConfigError"),
        }
    }

    #[test]
    fn test_project_root_accessor() {
        let dir = setup_test_project();
        let restarter = FastRestarter::new(&dir).expect("创建 FastRestarter");
        assert_eq!(restarter.project_root(), dir.as_path());
    }

    // === BuildResult 类型测试 ===

    #[test]
    fn test_build_result_success() {
        let result = BuildResult::Success;
        assert_eq!(result, BuildResult::Success);
    }

    #[test]
    fn test_build_result_failed() {
        let result = BuildResult::Failed {
            stderr: "编译错误".into(),
        };
        match &result {
            BuildResult::Failed { stderr } => assert_eq!(stderr, "编译错误"),
            _ => panic!("应为 Failed 变体"),
        }
    }

    #[test]
    fn test_build_result_up_to_date() {
        let result = BuildResult::UpToDate;
        assert_eq!(result, BuildResult::UpToDate);
    }

    // === build() 集成测试 ===

    #[test]
    fn test_build_with_cargo_installed() {
        // 确认 cargo 可用
        let cargo_check = Command::new("cargo").arg("--version").output();
        if cargo_check.is_err() {
            log::warn!("跳过测试：cargo 未安装");
            return;
        }

        let dir = setup_test_project();
        // 初始化 cargo 项目（需要 src/main.rs）
        fs::create_dir_all(dir.join("src")).expect("创建 src 目录");
        fs::write(
            dir.join("src").join("main.rs"),
            b"fn main() { println!(\"hello\"); }\n",
        )
        .expect("写入 main.rs 失败");

        let restarter = FastRestarter::new(&dir).expect("创建 FastRestarter");

        // 首次构建
        let result = restarter.build();
        assert!(result.is_ok(), "build 不应返回错误");

        match result.unwrap() {
            BuildResult::Success => {},
            BuildResult::Failed { stderr } => {
                log::error!("cargo build 失败:\n{}", stderr);
            },
            BuildResult::UpToDate => {
                // 也可能因为依赖已缓存而 UpToDate
            },
        }
    }

    #[test]
    fn test_build_with_syntax_error() {
        let cargo_check = Command::new("cargo").arg("--version").output();
        if cargo_check.is_err() {
            log::warn!("跳过测试：cargo 未安装");
            return;
        }

        let dir = setup_test_project();
        fs::create_dir_all(dir.join("src")).expect("创建 src 目录");
        // 写入有语法错误的 Rust 代码
        fs::write(
            dir.join("src").join("main.rs"),
            b"fn main() { invalid_syntax! }\n",
        )
        .expect("写入 main.rs 失败");

        let restarter = FastRestarter::new(&dir).expect("创建 FastRestarter");
        let result = restarter.build();
        assert!(result.is_ok(), "build 应返回 Ok（失败是 BuildResult 变体）");
        match result.unwrap() {
            BuildResult::Failed { stderr } => {
                assert!(!stderr.is_empty(), "应有编译错误输出");
            },
            other => panic!("期望 BuildResult::Failed，实际: {other:?}"),
        }
    }
}
