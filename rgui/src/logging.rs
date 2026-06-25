//! 日志系统初始化模块
//!
//! 提供统一的日志初始化入口 [`init_logging()`]，支持:
//! - 双通道输出（终端 stderr + 滚动日志文件）
//! - 按天滚动的日志文件（基于 tracing-appender）
//! - `logging.toml` 配置文件驱动
//! - `RUST_LOG` 环境变量覆盖日志级别
//! - 高频日志（`rgui::perf::*` target）独立开关

use serde::Deserialize;
use std::fmt;
use std::path::Path;
use std::sync::Once;
use time::OffsetDateTime;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// ---------------------------------------------------------------------------
// 配置结构体
// ---------------------------------------------------------------------------

/// 日志系统顶层配置
#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    /// 日志级别: `"trace"` | `"debug"` | `"info"` | `"warn"` | `"error"`
    #[serde(default = "default_level")]
    pub level: String,
    /// 文件输出配置
    #[serde(default)]
    pub file: FileConfig,
    /// 高频日志（perf）开关配置
    #[serde(default)]
    pub perf: PerfConfig,
}

/// 文件日志配置
#[derive(Debug, Clone, Deserialize)]
pub struct FileConfig {
    /// 日志输出目录，相对于当前工作目录
    #[serde(default = "default_directory")]
    pub directory: String,
    /// 滚动策略（当前仅支持 `"daily"`）
    #[serde(default = "default_rotation")]
    pub rotation: String,
    /// 保留的日志文件最大数量
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

/// 高频日志（perf）配置
///
/// 高频日志使用 target 前缀 `rgui::perf::`，独立于全局日志级别受此开关控制。
#[derive(Debug, Clone, Deserialize)]
pub struct PerfConfig {
    /// 是否启用高频日志（需全局 level ≤ debug 才实际生效）
    #[serde(default)]
    pub enabled: bool,
    /// 缓冲区大小（KB），保留字段供后续批量写入使用
    #[serde(default = "default_buffer_kb")]
    pub buffer_kb: usize,
}

// ---- 为 serde(default) 提供默认值函数 ----

fn default_level() -> String {
    "info".to_string()
}
fn default_directory() -> String {
    "logs".to_string()
}
fn default_rotation() -> String {
    "daily".to_string()
}
const fn default_max_files() -> usize {
    7
}
const fn default_buffer_kb() -> usize {
    8
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            file: FileConfig::default(),
            perf: PerfConfig::default(),
        }
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            directory: default_directory(),
            rotation: default_rotation(),
            max_files: default_max_files(),
        }
    }
}

impl Default for PerfConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_kb: default_buffer_kb(),
        }
    }
}

// ---------------------------------------------------------------------------
// 配置加载
// ---------------------------------------------------------------------------

impl LogConfig {
    /// 从 `logging.toml` 加载配置。
    ///
    /// 返回 `(config, found)`：
    /// - `found == true`：配置文件存在且解析成功
    /// - `found == false`：文件不存在或解析失败，使用默认配置
    fn load() -> (Self, bool) {
        let path = Path::new("logging.toml");
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<LogConfig>(&content) {
                Ok(cfg) => (cfg, true),
                Err(e) => {
                    eprintln!("[WARN] 解析 logging.toml 失败: {}，使用默认配置", e);
                    (Self::default(), false)
                },
            },
            Err(_) => (Self::default(), false),
        }
    }
}

// ---------------------------------------------------------------------------
// 自定义时间格式化器（实现 FormatTime trait）
// ---------------------------------------------------------------------------

/// 终端时间格式：`[HH:MM:SS]`（UTC）
struct TerminalTimer;

impl FormatTime for TerminalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = OffsetDateTime::now_utc();
        write!(
            w,
            "[{:02}:{:02}:{:02}]",
            now.hour(),
            now.minute(),
            now.second()
        )
    }
}

/// 文件时间格式：`[YYYY-MM-DD HH:MM:SS]`（UTC）
struct FileTimer;

impl FormatTime for FileTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = OffsetDateTime::now_utc();
        write!(
            w,
            "[{:04}-{:02}-{:02} {:02}:{:02}:{:02}]",
            now.year(),
            u8::from(now.month()),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        )
    }
}

// ---------------------------------------------------------------------------
// 初始化入口
// ---------------------------------------------------------------------------

static INIT: Once = Once::new();

/// 初始化日志系统（全局最多执行一次，多次调用安全无副作用）。
///
/// # 行为
///
/// 1. 读取 `logging.toml`（不存在则使用默认配置）
/// 2. 构建双 Layer tracing subscriber：
///    - **终端 Layer**（stderr）：格式 `[HH:MM:SS] 级别: 消息`，无颜色，不显示 target
///    - **文件 Layer**（滚动文件）：格式 `[YYYY-MM-DD HH:MM:SS] [级别] [target] 消息`
/// 3. 高频日志（`rgui::perf::*` target）受 `perf.enabled` 独立控制
/// 4. `RUST_LOG` 环境变量优先于配置文件中的 `level`
///
/// # Panics
///
/// 仅在无法创建日志目录或无法初始化滚动文件 appender 时 panic。
pub fn init_logging() {
    INIT.call_once(|| {
        let (config, config_found) = LogConfig::load();

        // ---- EnvFilter：RUST_LOG 环境变量优先 ----
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // 只对 rgui target 启用用户配置的 level，其他依赖库保持 info
            // 避免 wgpu/naga/winit 等底层库的 DEBUG 日志淹没输出
            let filter_str = format!("rgui={},info", config.level);
            EnvFilter::new(&filter_str)
        });

        // ---- 高频日志（perf）开关过滤器 ----
        let perf_enabled = config.perf.enabled;
        let perf_filter = FilterFn::new(move |metadata| {
            if metadata.target().starts_with("rgui::perf::") {
                return perf_enabled;
            }
            // 放行所有非 perf 日志
            true
        });

        // ---- 文件 Appender（按天滚动） ----
        // 确保日志目录存在
        std::fs::create_dir_all(&config.file.directory).expect("无法创建日志目录");

        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("rgui")
            .filename_suffix("log")
            .max_log_files(config.file.max_files)
            .build(&config.file.directory)
            .expect("初始化滚动文件 appender 失败");

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        // WorkerGuard 必须存活以保持后台写入线程运行；
        // 泄漏它以获得 'static 生命周期（进程退出时线程自然终止）
        std::mem::forget(guard);

        // ---- 终端 Layer（stderr） ----
        let terminal_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_ansi(false)
            .with_timer(TerminalTimer)
            .with_writer(std::io::stderr);

        // ---- 文件 Layer ----
        let file_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(false)
            .with_timer(FileTimer)
            .with_writer(non_blocking);

        // ---- 组装 subscriber 并初始化 ----
        tracing_subscriber::registry()
            .with(env_filter)
            .with(perf_filter)
            .with(terminal_layer)
            .with(file_layer)
            .init();

        // ---- 初始化后的诊断日志 ----
        if !config_found {
            tracing::warn!("未找到 logging.toml，使用默认配置");
        }

        tracing::info!(
            "日志系统已初始化，级别={}，输出目录={}",
            config.level,
            config.file.directory
        );

        // perf 启用但全局级别不足时的警告
        let has_rust_log = std::env::var("RUST_LOG").is_ok();
        let level_is_debug_or_lower = matches!(config.level.as_str(), "trace" | "debug");
        if config.perf.enabled && !has_rust_log && !level_is_debug_or_lower {
            tracing::warn!("perf 日志需要 debug 级别，当前为 {}", config.level);
        }
    });
}
