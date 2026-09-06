//! 生产级日志初始化（D22）。
//!
//! tracing 同步面 + 双输出流（物理隔离）：
//! - **库日志**（target 非 `rgui_test_signal`）→ **stderr**（开发者排障，带级别/时间戳）
//! - **测试信号**（target = `rgui_test_signal`）→ **stdout**（纯裸 message = 原 token 不变，无级别/时间戳/前缀）
//!
//! 级别由 `RUST_LOG`（简化解析）精确开关：
//! - `RUST_LOG=off` → 全关（性能基准，帧路径零日志）
//! - `RUST_LOG=info|debug|warn|error|trace` → 对应全局级别（默认 `info`）
//! - `RUST_LOG` 含 `rgui_test_signal` → **signal_only**（只出测试信号，库日志关；D21 脚本干净）
//!
//! 注册幂等（`Once`），由 facade `App::run` 首次调用（demo/bin 最早入口）。无 async。

use std::io;
use std::sync::Once;

use tracing_subscriber::filter::{filter_fn, LevelFilter};
use tracing_subscriber::prelude::*;

static INIT: Once = Once::new();
const SIGNAL_TARGET: &str = "rgui_test_signal";

/// 初始化全局日志（幂等）。库日志→stderr；`rgui_test_signal` 测试信号→stdout（纯 message）。
pub fn init_logging() {
    INIT.call_once(|| {
        // 解析 RUST_LOG 级别；默认 info。
        let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
        let level = match rust_log.trim() {
            "off" => LevelFilter::OFF,
            "error" => LevelFilter::ERROR,
            "warn" => LevelFilter::WARN,
            "debug" => LevelFilter::DEBUG,
            "trace" => LevelFilter::TRACE,
            _ => LevelFilter::INFO, // 默认 info + 含 "rgui_test_signal=..." 落地 info
        };
        // signal_only：RUST_LOG 含 rgui_test_signal → 只出测试信号（库日志关）。
        let lib_on = !rust_log.contains("rgui_test_signal");

        // 库日志：stderr，排除测试信号 target；级别按 RUST_LOG（lib_on 时）。
        let lib_filter = {
            let lvl = level;
            filter_fn(move |m: &tracing::Metadata<'_>| {
                lib_on && m.target() != SIGNAL_TARGET && m.level() <= &lvl
            })
        };
        let lib_layer = tracing_subscriber::fmt::layer()
            .with_writer(io::stderr)
            .with_ansi(false)
            .with_filter(lib_filter);

        // 测试信号：stdout，纯裸 message（无 level/target/时间戳/颜色）→ D21 脚本正则零改动。
        let signal_filter = filter_fn(move |m: &tracing::Metadata<'_>| {
            m.target() == SIGNAL_TARGET && m.level() <= &level
        });
        let signal_layer = tracing_subscriber::fmt::layer()
            .with_writer(io::stdout)
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_level(false)
            .with_filter(signal_filter);

        tracing_subscriber::registry()
            .with(lib_layer)
            .with(signal_layer)
            .init();
    });
}
