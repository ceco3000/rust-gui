//! 路径细分缓存类型。
//!
//! 为复杂 SVG 路径的 GPU 细分提供缓存。阶段 2 前为类型占位。

/// 路径细分缓存占位类型。
///
/// 用于 `RenderLayoutCache::path_tessellation` 字段，缓存复杂 SVG 路径
/// 的三角形/曲线细分结果以避免每帧重复计算。
///
/// 阶段 2 将扩展为包含细分顶点/索引缓冲区的完整实现。
///
/// # 示例
///
/// ```
/// use rgui_render::PathTessellation;
/// let t = PathTessellation::default();
/// ```
#[derive(Debug, Clone, Default)]
pub struct PathTessellation;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_tessellation_default() {
        let t = PathTessellation::default();
        let t2 = t.clone();
        // 占位类型应为同一逻辑值
        assert_eq!(format!("{t:?}"), format!("{t2:?}"));
    }

    #[test]
    fn path_tessellation_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PathTessellation>();
    }
}
