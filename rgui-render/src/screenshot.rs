//! 截图回归测试框架（D9 §5、D8 T03）。
//!
//! 提供基于像素级比较的截图回归测试基础设施：
//! - CIE Lab 颜色空间中的 ΔE 距离阈值比较
//! - 可配置的匹配率和颜色差异容差
//! - 基准 PNG 管理（首次运行保存基准，后续运行比较）
//!
//! # 设计
//!
//! 遵循 D8 T03 的验收标准：
//! - 像素匹配率 ≥ 99.5%（`ScreenshotTolerance::default().match_rate`）
//! - CIE76 ΔE ≤ 1.0（`ScreenshotTolerance::default().delta_e_threshold`）
//! - 基准 PNG 存放在 `tests/screenshots/baseline/`，实际输出存放在
//!   `tests/screenshots/actual/`
//!
//! # 示例
//!
//! ```ignore
//! use rgui_render::offscreen::OffscreenTestRunner;
//! use rgui_render::screenshot::assert_screenshot_matches;
//! use rgui_render::scene::SceneGraph;
//!
//! let mut runner = OffscreenTestRunner::new(200, 100);
//! let scene = SceneGraph::new(1);
//! assert_screenshot_matches(&mut runner, &scene, "empty_scene");
//! ```

#[cfg(feature = "offscreen")]
use std::path::PathBuf;

#[cfg(feature = "offscreen")]
use crate::offscreen::OffscreenTestRunner;
#[cfg(feature = "offscreen")]
use crate::scene::SceneGraph;

// ============================================================================
// 路径常量
// ============================================================================

/// 基准截图存放的目录（相对于 Cargo workspace 根目录）。
#[cfg(feature = "offscreen")]
const BASELINE_DIR: &str = "tests/screenshots/baseline";

/// 实际截图存放的目录（相对于 Cargo workspace 根目录）。
#[cfg(feature = "offscreen")]
const ACTUAL_DIR: &str = "tests/screenshots/actual";

// ============================================================================
// ScreenshotTolerance -- 容差配置
// ============================================================================

/// 截图回归测试的容差配置（D8 T03）。
///
/// 控制像素级比较的接受阈值。
///
/// | 预设 | 匹配率 | ΔE | 适用场景 |
/// |------|--------|-----|---------|
/// | [`ScreenshotTolerance::default()`] | 99.5% | 1.0 | 标准（抗锯齿容差） |
/// | [`ScreenshotTolerance::relaxed()`] | 95.0% | 3.0 | 跨平台 GPU 差异 |
/// | [`ScreenshotTolerance::strict()`] | 99.9% | 0.5 | CPU 光栅化精确比对 |
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenshotTolerance {
    /// 像素匹配率阈值，取值范围 [0.0, 1.0]。
    ///
    /// 默认 0.995（99.5%），允许约 0.5% 像素因抗锯齿而产生微小偏差。
    pub match_rate: f64,
    /// CIE76 ΔE 颜色差异阈值。
    ///
    /// 默认 1.0。ΔE ≤ 1.0 的像素颜色差异人眼难以察觉。
    pub delta_e_threshold: f64,
}

impl Default for ScreenshotTolerance {
    fn default() -> Self {
        Self {
            match_rate: 0.995,
            delta_e_threshold: 1.0,
        }
    }
}

impl ScreenshotTolerance {
    /// 创建宽松容差配置。
    ///
    /// 95% 匹配率 + ΔE ≤ 3.0。适用于跨 GPU 后端的截图比对。
    #[must_use]
    pub const fn relaxed() -> Self {
        Self {
            match_rate: 0.95,
            delta_e_threshold: 3.0,
        }
    }

    /// 创建严格容差配置。
    ///
    /// 99.9% 匹配率 + ΔE ≤ 0.5。适用于 CPU 光栅化确定性输出比对。
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            match_rate: 0.999,
            delta_e_threshold: 0.5,
        }
    }
}

// ============================================================================
// CIE76 ΔE 颜色距离
// ============================================================================

/// 将 sRGB 字节值转换为 CIE Lab (L*, a*, b*)。
///
/// 转换路径：sRGB → linear sRGB → CIE XYZ (D65) → CIE Lab。
/// sRGB ↔ linear 传输函数复用 [`rgui_core::Color::to_linear`]。
fn srgb_bytes_to_lab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let color = rgui_core::Color::new(
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
        1.0,
    );
    let linear = color.to_linear();

    // linear sRGB → CIE XYZ (sRGB D65 标准矩阵)
    let x = linear
        .r
        .mul_add(0.4124564, linear.g.mul_add(0.3575761, linear.b * 0.1804375));
    let y = linear
        .r
        .mul_add(0.2126729, linear.g.mul_add(0.7151522, linear.b * 0.0721750));
    let z = linear
        .r
        .mul_add(0.0193339, linear.g.mul_add(0.1191920, linear.b * 0.9503041));

    xyz_to_lab(x, y, z)
}

/// CIE XYZ (D65) → CIE Lab 转换（CIE 1976 标准）。
fn xyz_to_lab(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    // D65 参考白点（2-degree observer）
    const XN: f64 = 0.95047;
    const YN: f64 = 1.0;
    const ZN: f64 = 1.08883;

    let fx = xyz_lab_f(x / XN);
    let fy = xyz_lab_f(y / YN);
    let fz = xyz_lab_f(z / ZN);

    let l = 116.0f64.mul_add(fy, -16.0);
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    (l, a, b)
}

/// CIE Lab 转换中使用的非线性辅助函数。
fn xyz_lab_f(t: f64) -> f64 {
    if t > 0.008856 {
        t.powf(1.0 / 3.0)
    } else {
        t.mul_add(7.787, 16.0 / 116.0)
    }
}

/// 计算两个 sRGB 像素之间的 CIE76 ΔE 颜色距离。
///
/// 返回非负值：0.0 表示颜色完全相同，值越大表示差异越明显。
/// 参考尺度：
/// - ΔE ≤ 1.0：人眼难以察觉
/// - ΔE ≤ 2.0：紧密观察可察觉
/// - ΔE ≤ 10.0：明显色差
///
/// # 示例
///
/// ```ignore
/// use rgui_render::screenshot::delta_e;
///
/// // 相同颜色
/// let d = delta_e(255, 0, 0, 255, 0, 0);
/// assert!(d < 0.001);
///
/// // 不同颜色
/// let d = delta_e(255, 0, 0, 0, 255, 0);
/// assert!(d > 10.0);
/// ```
#[must_use]
pub fn delta_e(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> f64 {
    let (l1, a1, b1) = srgb_bytes_to_lab(r1, g1, b1);
    let (l2, a2, b2) = srgb_bytes_to_lab(r2, g2, b2);
    ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
}

// ============================================================================
// 像素差异比较
// ============================================================================

/// 计算两个 RGBA8888 像素缓冲区之间的差异率。
///
/// 逐像素比较颜色距离，统计超出 ΔE 阈值的像素占比。
///
/// # 参数
/// - `baseline`: 基准像素缓冲区（RGBA 交错，行优先）
/// - `actual`: 实际渲染输出（与 `baseline` 等长）
/// - `tolerance`: 容差配置
///
/// # 返回值
/// 区间 [0.0, 1.0] 的差异率。0.0 = 完美匹配；0.005 = 0.5% 像素不同。
///
/// # Panics
/// - 如果两个缓冲区长度不一致
/// - 如果缓冲区长度不是 4 的倍数（非完整 RGBA 像素）
#[must_use]
pub fn pixel_diff_ratio(baseline: &[u8], actual: &[u8], tolerance: &ScreenshotTolerance) -> f64 {
    assert_eq!(
        baseline.len(),
        actual.len(),
        "像素缓冲区大小不一致：基准 {} 字节，实际 {} 字节",
        baseline.len(),
        actual.len()
    );
    assert_eq!(
        baseline.len() % 4,
        0,
        "像素缓冲区长度必须是 4 的倍数（RGBA），当前为 {} 字节",
        baseline.len()
    );

    let pixel_count = baseline.len() / 4;
    if pixel_count == 0 {
        return 0.0;
    }

    let different = baseline
        .chunks_exact(4)
        .zip(actual.chunks_exact(4))
        .filter(|(bp, ap)| {
            delta_e(bp[0], bp[1], bp[2], ap[0], ap[1], ap[2]) > tolerance.delta_e_threshold
        })
        .count();

    different as f64 / pixel_count as f64
}

// ============================================================================
// 截图回归断言
// ============================================================================

/// 截图回归断言，使用默认容差（99.5% 匹配率 + ΔE ≤ 1.0）。
///
/// # 行为
///
/// 1. 将当前场景渲染为 PNG，保存到 `tests/screenshots/actual/{test_name}.png`
/// 2. 如果 `tests/screenshots/baseline/{test_name}.png` 存在：
///    - 加载基准 PNG，逐像素比较
///    - 差异率超出阈值时 panic
/// 3. 如果基准不存在：
///    - 将本次渲染输出复制为基准 PNG（首次基线建立）
///
/// # Panics
///
/// - 渲染失败
/// - 基准 PNG 尺寸与实际输出不匹配
/// - 像素差异率超出容差阈值
/// - 文件 I/O 错误
///
/// # 示例
///
/// ```ignore
/// use rgui_render::offscreen::OffscreenTestRunner;
/// use rgui_render::screenshot::assert_screenshot_matches;
/// use rgui_render::scene::SceneGraph;
///
/// let mut runner = OffscreenTestRunner::new(200, 100);
/// let scene = SceneGraph::new(1);
/// assert_screenshot_matches(&mut runner, &scene, "empty_scene");
/// ```
#[cfg(feature = "offscreen")]
pub fn assert_screenshot_matches(
    runner: &mut OffscreenTestRunner,
    scene: &SceneGraph,
    test_name: &str,
) {
    assert_screenshot_matches_with_tolerance(
        runner,
        scene,
        test_name,
        &ScreenshotTolerance::default(),
    );
}

/// 截图回归断言，指定容差配置。
///
/// 与 [`assert_screenshot_matches`] 行为相同，但允许自定义容差。
///
/// # Panics
/// 同 [`assert_screenshot_matches`]。
#[cfg(feature = "offscreen")]
pub fn assert_screenshot_matches_with_tolerance(
    runner: &mut OffscreenTestRunner,
    scene: &SceneGraph,
    test_name: &str,
    tolerance: &ScreenshotTolerance,
) {
    let baseline_dir = PathBuf::from(BASELINE_DIR);
    let actual_dir = PathBuf::from(ACTUAL_DIR);

    std::fs::create_dir_all(&baseline_dir)
        .unwrap_or_else(|e| panic!("无法创建基准截图目录 {}: {e}", baseline_dir.display()));
    std::fs::create_dir_all(&actual_dir)
        .unwrap_or_else(|e| panic!("无法创建实际截图目录 {}: {e}", actual_dir.display()));

    let baseline_path = baseline_dir.join(format!("{test_name}.png"));
    let actual_path = actual_dir.join(format!("{test_name}.png"));

    runner
        .render_to_png(scene, &actual_path)
        .unwrap_or_else(|e| panic!("渲染截图失败: {e}"));

    if baseline_path.exists() {
        let baseline_img = image::open(&baseline_path)
            .unwrap_or_else(|e| panic!("读取基准截图失败 {}: {e}", baseline_path.display()));
        let actual_img = image::open(&actual_path)
            .unwrap_or_else(|e| panic!("读取实际截图失败 {}: {e}", actual_path.display()));

        let (bw, bh) = (baseline_img.width(), baseline_img.height());
        let (aw, ah) = (actual_img.width(), actual_img.height());
        assert_eq!(
            (bw, bh),
            (aw, ah),
            "截图尺寸不匹配：基准 {bw}x{bh}，实际 {aw}x{ah}"
        );

        let baseline_rgba = baseline_img.into_rgba8();
        let actual_rgba = actual_img.into_rgba8();

        let ratio = pixel_diff_ratio(baseline_rgba.as_raw(), actual_rgba.as_raw(), tolerance);
        assert!(
            ratio <= (1.0 - tolerance.match_rate),
            "截图回归失败：{:.2}% 像素不同（阈值 {:.2}%）",
            ratio * 100.0,
            (1.0 - tolerance.match_rate) * 100.0,
        );
    } else {
        std::fs::copy(&actual_path, &baseline_path).unwrap_or_else(|e| {
            panic!(
                "保存基准截图失败 {} → {}: {e}",
                actual_path.display(),
                baseline_path.display()
            )
        });
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // delta_e 测试
    // ------------------------------------------------------------------

    #[test]
    fn delta_e_identical_colors_is_zero() {
        let d = delta_e(128, 128, 128, 128, 128, 128);
        assert!(d < 0.001, "相同颜色的 ΔE 应接近 0，实际 {d:.6}");
    }

    #[test]
    fn delta_e_same_with_zero() {
        // 黑色与黑色
        let d = delta_e(0, 0, 0, 0, 0, 0);
        assert!(d < 0.001);
    }

    #[test]
    fn delta_e_same_with_white() {
        let d = delta_e(255, 255, 255, 255, 255, 255);
        assert!(d < 0.001);
    }

    #[test]
    fn delta_e_red_vs_green_is_large() {
        // 红 vs 绿——对比鲜明的颜色差距应 > 10
        let d = delta_e(255, 0, 0, 0, 255, 0);
        assert!(d > 10.0, "红 vs 绿的 ΔE 应 > 10，实际 {d:.2}");
    }

    #[test]
    fn delta_e_one_lsb_is_small() {
        // 单比特差异应 ≤ 1.0
        let d = delta_e(128, 128, 128, 129, 128, 128);
        assert!(d <= 1.0, "单比特差异的 ΔE 应 ≤ 1.0，实际 {d:.4}");
    }

    #[test]
    fn delta_e_blue_vs_yellow_is_large() {
        // 蓝 vs 黄——互补色差距应 > 30
        let d = delta_e(0, 0, 255, 255, 255, 0);
        assert!(d > 30.0, "蓝 vs 黄的 ΔE 应 > 30，实际 {d:.2}");
    }

    #[test]
    fn delta_e_commutative() {
        // ΔE 应对称
        let d1 = delta_e(200, 100, 50, 50, 100, 200);
        let d2 = delta_e(50, 100, 200, 200, 100, 50);
        let diff = (d1 - d2).abs();
        assert!(diff < 0.001, "ΔE 对换序不对称：{d1:.6} vs {d2:.6}");
    }

    // ------------------------------------------------------------------
    // pixel_diff_ratio 测试
    // ------------------------------------------------------------------

    #[test]
    fn pixel_diff_identical_buffers() {
        let buf: Vec<u8> = (0..16).map(|i| i as u8).collect(); // 4 像素
        let tolerance = ScreenshotTolerance::default();
        let ratio = pixel_diff_ratio(&buf, &buf, &tolerance);
        assert_eq!(ratio, 0.0, "相同缓冲区差异率应为 0.0");
    }

    #[test]
    fn pixel_diff_empty_buffer() {
        let tolerance = ScreenshotTolerance::default();
        let ratio = pixel_diff_ratio(&[], &[], &tolerance);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn pixel_diff_completely_different() {
        // 4 个红色像素 vs 4 个蓝色像素——应全部不同
        let baseline: Vec<u8> = (0..4).flat_map(|_| [255u8, 0, 0, 255]).collect();
        let actual: Vec<u8> = (0..4).flat_map(|_| [0u8, 0, 255, 255]).collect();
        let tolerance = ScreenshotTolerance::default();
        let ratio = pixel_diff_ratio(&baseline, &actual, &tolerance);
        assert_eq!(ratio, 1.0, "完全不同的缓冲区差异率应为 1.0");
    }

    #[test]
    fn pixel_diff_relaxed_tolerance_reduces_difference() {
        // 单比特差异在宽松容差下可能被忽略
        let baseline: Vec<u8> = vec![128, 128, 128, 255, 128, 128, 128, 255];
        let actual: Vec<u8> = vec![129, 128, 128, 255, 128, 129, 128, 255];
        let strict = ScreenshotTolerance::strict();
        let relaxed = ScreenshotTolerance::relaxed();
        let ratio_strict = pixel_diff_ratio(&baseline, &actual, &strict);
        let ratio_relaxed = pixel_diff_ratio(&baseline, &actual, &relaxed);
        // 严格容差下应检出差异，宽松容差下可能容忍
        assert!(ratio_relaxed <= ratio_strict);
    }

    #[test]
    #[should_panic(expected = "像素缓冲区大小不一致")]
    fn pixel_diff_mismatched_size_panics() {
        let a = vec![0u8; 8];
        let b = vec![0u8; 4];
        let _ = pixel_diff_ratio(&a, &b, &ScreenshotTolerance::default());
    }

    #[test]
    #[should_panic(expected = "像素缓冲区长度必须是 4 的倍数")]
    fn pixel_diff_non_multiple_of_four_panics() {
        let buf = vec![0u8; 9]; // 2 像素 + 1 残留字节
        let _ = pixel_diff_ratio(&buf, &buf, &ScreenshotTolerance::default());
    }

    // ------------------------------------------------------------------
    // ScreenshotTolerance 测试
    // ------------------------------------------------------------------

    #[test]
    fn default_tolerance_meets_d8_spec() {
        let t = ScreenshotTolerance::default();
        assert_eq!(t.match_rate, 0.995);
        assert_eq!(t.delta_e_threshold, 1.0);
    }

    #[test]
    fn relaxed_tolerance_is_looser() {
        let t = ScreenshotTolerance::relaxed();
        assert_eq!(t.match_rate, 0.95);
        assert_eq!(t.delta_e_threshold, 3.0);
    }

    #[test]
    fn strict_tolerance_is_tighter() {
        let t = ScreenshotTolerance::strict();
        assert_eq!(t.match_rate, 0.999);
        assert_eq!(t.delta_e_threshold, 0.5);
    }

    // ------------------------------------------------------------------
    // CIE Lab 转换测试
    // ------------------------------------------------------------------

    #[test]
    fn srgb_white_to_lab() {
        // 标准 D65 白色 ≈ L*=100, a*≈0, b*≈0
        let (l, a, b) = srgb_bytes_to_lab(255, 255, 255);
        assert!(l > 99.0 && l < 101.0, "白色 L* 应 ≈ 100，实际 {l:.2}");
        assert!(a.abs() < 1.0, "白色 a* 应 ≈ 0，实际 {a:.2}");
        assert!(b.abs() < 1.0, "白色 b* 应 ≈ 0，实际 {b:.2}");
    }

    #[test]
    fn srgb_black_to_lab() {
        let (l, _a, _b) = srgb_bytes_to_lab(0, 0, 0);
        assert!(l < 1.0, "黑色 L* 应 ≈ 0，实际 {l:.2}");
    }

    #[test]
    fn srgb_red_to_lab_positive_a() {
        // 红色应在 CIE a* 轴正方向
        let (_l, a, _b) = srgb_bytes_to_lab(255, 0, 0);
        assert!(a > 30.0, "红色 a* 应 > 30，实际 {a:.2}");
    }

    #[test]
    fn srgb_green_to_lab_negative_a() {
        // 绿色应在 CIE a* 轴负方向
        let (_l, a, _b) = srgb_bytes_to_lab(0, 255, 0);
        assert!(a < -30.0, "绿色 a* 应 < -30，实际 {a:.2}");
    }

    #[test]
    fn srgb_blue_to_lab_negative_b() {
        // 蓝色应在 CIE b* 轴负方向
        let (_l, _a, b) = srgb_bytes_to_lab(0, 0, 255);
        assert!(b < -30.0, "蓝色 b* 应 < -30，实际 {b:.2}");
    }

    #[test]
    fn srgb_yellow_to_lab_positive_b() {
        // 黄色（红+绿=255,255,0）应在 CIE b* 轴正方向
        let (_l, _a, b) = srgb_bytes_to_lab(255, 255, 0);
        assert!(b > 30.0, "黄色 b* 应 > 30，实际 {b:.2}");
    }
}
