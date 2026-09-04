//! 路径细分模块（GPU 资源类型，契约 §2.4）。
//! D3 阶段 0：占位类型定义。

/// 路径细分结果。
#[derive(Debug, Clone, Default)]
pub struct PathTessellation {
    _marker: std::marker::PhantomData<()>,
}
