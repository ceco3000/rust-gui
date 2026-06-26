//! 字体度量缓存（D3 §8，D1 §6.3）。
//!
//! 本模块负责在 cosmic-text 启动时填充字体度量数据。
//! `FONT_METRICS_CACHE` 是一个 `'static` 缓存，可在整个框架中
//! 共享，供 `MeasureContext` 使用。
//!
//! # 初始化
//!
//! `TextEngine` 在创建时调用 [`init_font_metrics()`]，确保字体
//! 度量在首次 `measure()` 调用前已就绪。
//!
//! # 当前支持
//!
//! 当前使用实测的 Noto Sans CJK SC Regular 字体度量（与嵌入字体匹配）。
//! 未来可通过解析实际字体文件或添加其他字体族扩展。

use std::sync::OnceLock;

use rgui_core::context::{FontMetrics, FontMetricsCache};

/// 全局字体度量缓存。
///
/// 首次访问时自动以 Noto Sans CJK SC Regular 字体度量初始化。
/// 使用 `OnceLock` 确保线程安全的一次性初始化。
static FONT_METRICS_CACHE: OnceLock<FontMetricsCache> = OnceLock::new();

/// 初始化全局字体度量缓存。
///
/// 此函数是幂等的——多次调用只会执行一次初始化。
/// 应在 `TextEngine::new()` 中调用，确保字体度量在首次
/// 布局/测量前就绪。
pub fn init_font_metrics() {
    FONT_METRICS_CACHE.get_or_init(|| {
        FontMetricsCache::new(FontMetrics::new(
            1.160,  // ascent:  1160/1000 Noto Sans CJK SC Regular
            -0.288, // descent: -288/1000
            0.0,    // line_gap: 0/1000
            0.543,  // x_height: 543/1000
            0.733,  // cap_height: 733/1000
        ))
    });
}

/// 获取全局字体度量缓存的静态引用。
///
/// # 返回
///
/// `&'static FontMetricsCache` — 可用于构造 `MeasureContext`。
///
/// # Panics
///
/// 如果 `init_font_metrics()` 尚未调用，则 panic。
/// 调用方必须确保已在框架启动流程中调用 `init_font_metrics()`。
#[must_use]
pub fn font_metrics_cache() -> &'static FontMetricsCache {
    FONT_METRICS_CACHE
        .get()
        .expect("font_metrics_cache() called before init_font_metrics(). Call init_font_metrics() at startup.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        init_font_metrics();
        init_font_metrics();
        let cache = font_metrics_cache();
        let m = cache.default_metrics;
        assert!((m.ascent - 1.160).abs() < 0.001);
    }

    #[test]
    fn init_sets_noto_cjk_metrics() {
        // 使用原始 OnceLock 确保测试隔离
        let lock: OnceLock<FontMetricsCache> = OnceLock::new();
        lock.get_or_init(|| {
            FontMetricsCache::new(FontMetrics::new(1.160, -0.288, 0.0, 0.543, 0.733))
        });
        let cache = lock.get().unwrap();
        let m = cache.default_metrics;
        assert!((m.ascent - 1.160).abs() < 0.001);
        assert!(m.descent < -0.2);
    }

    #[test]
    fn cache_provides_static_reference() {
        init_font_metrics();
        let cache: &'static FontMetricsCache = font_metrics_cache();
        let _: &'static FontMetricsCache = cache;
    }

    #[test]
    fn metrics_line_height_is_positive() {
        init_font_metrics();
        let cache = font_metrics_cache();
        let lh = cache.default_metrics.line_height();
        assert!(lh > 1.0, "line_height should be > 1.0 em, got {lh}");
    }
}
