//! 渲染后端工厂（D3 §7.1）。
//!
//! 提供后端自动选择（Vello > Skia > 错误）和指定后端创建功能。
//!
//! # 架构
//!
//! - [`RenderBackendFactory`] 是渲染后端的单一入口点。
//! - 后端选择策略通过 `create()` 方法自动执行。
//! - 可通过 `create_backend()` 显式指定后端类型。
//! - 可通过 `available_backends()` 查询当前编译可用的后端列表。

use crate::backend::{RenderBackend, RenderError, RenderParams};

#[cfg(feature = "skia-backend")]
use crate::SkiaBackend;

/// 渲染后端类型枚举。
///
/// 标识支持的渲染后端类型，用于工厂查询和显式选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendType {
    /// Vello GPU 矢量渲染后端。
    ///
    /// 主推后端，依赖 wgpu + Vello，需要 GPU 支持。
    /// 实现见 R05 任务（当前为占位，`create_backend` 返回错误）。
    Vello,
    /// Skia CPU 光栅化后端。
    ///
    /// Fallback 后端，使用 skia-safe 在 CPU 上完成渲染。
    /// 适用于无 GPU 环境、CI 测试和离屏渲染。
    Skia,
}

impl BackendType {
    /// 返回后端的人类可读名称。
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Vello => "Vello",
            Self::Skia => "Skia",
        }
    }
}

/// 渲染后端工厂。
///
/// 提供后端自动选择和显式创建功能。
///
/// # 自动选择策略
///
/// `create()` 方法按以下优先级选择后端：
///
/// 1. **VelloBackend** — 需要 `vello-backend` feature（R05 实现）。
/// 2. **SkiaBackend** — 需要 `skia-backend` feature。
/// 3. 返回 [`RenderError::NoAvailableBackend`]。
///
/// # 示例
///
/// ```rust
/// use rgui_render::{RenderParams, RenderBackendFactory, BackendType};
///
/// let params = RenderParams::default();
/// // 创建首选可用后端。无可用后端时返回错误。
/// let _ = RenderBackendFactory::create(&params);
/// ```
pub struct RenderBackendFactory;

impl RenderBackendFactory {
    /// 创建首选可用的渲染后端。
    ///
    /// 按优先级（Vello > Skia）自动选择。
    /// 当前 VelloBackend 尚未实现（R05），实际仅返回 SkiaBackend。
    ///
    /// # 错误
    ///
    /// - [`RenderError::NoAvailableBackend`] — 所有后端均不可用。
    #[cfg_attr(not(feature = "skia-backend"), allow(unused_variables))]
    pub fn create(params: &RenderParams) -> Result<Box<dyn RenderBackend>, RenderError> {
        // VelloBackend 将在 R05 任务中实现。
        // 当前 VelloBackend 尚未创建，直接回退到 SkiaBackend。
        // 待 R05 完成后，在此处插入：
        //
        // ```rust
        // #[cfg(feature = "vello-backend")]
        // return Ok(Box::new(VelloBackend::new(...)?));
        // ```

        // 回退到 SkiaBackend
        #[cfg(feature = "skia-backend")]
        {
            let backend = SkiaBackend::new();
            // params 在此处不参与 SkiaBackend 构造，仅在 render() 时使用。
            // 保留引用以防未来需要传递初始化参数。
            let _ = params;
            Ok(Box::new(backend))
        }
        #[cfg(not(feature = "skia-backend"))]
        {
            // 无可用后端
            Err(RenderError::NoAvailableBackend)
        }
    }

    /// 创建指定类型的渲染后端。
    ///
    /// # 参数
    ///
    /// * `backend_type` — 要创建的后端类型（参见 [`BackendType`]）。
    /// * `params` — 渲染参数。
    ///
    /// # 错误
    ///
    /// - [`RenderError::UnsupportedBackend`] — 指定后端未编译或未实现。
    pub fn create_backend(
        backend_type: BackendType,
        _params: &RenderParams,
    ) -> Result<Box<dyn RenderBackend>, RenderError> {
        match backend_type {
            BackendType::Vello => {
                // VelloBackend 将在 R05 中实现
                Err(RenderError::UnsupportedBackend(
                    "VelloBackend not yet implemented (see R05)",
                ))
            },
            BackendType::Skia => {
                #[cfg(feature = "skia-backend")]
                {
                    let backend = SkiaBackend::new();
                    Ok(Box::new(backend))
                }
                #[cfg(not(feature = "skia-backend"))]
                Err(RenderError::UnsupportedBackend(
                    "skia-backend feature not enabled",
                ))
            },
        }
    }

    /// 查询当前编译可用的后端类型列表。
    ///
    /// 返回值按优先级排序（Vello > Skia）。
    #[must_use]
    pub fn available_backends() -> Vec<BackendType> {
        // VelloBackend 将在 R05 中实现后在此处添加至列表首位
        // #[cfg(feature = "vello-backend")]
        // backends.push(BackendType::Vello);

        #[cfg(feature = "skia-backend")]
        {
            vec![BackendType::Skia]
        }
        #[cfg(not(feature = "skia-backend"))]
        {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::SceneGraph;

    /// 辅助函数：创建一个用于测试的默认 RenderParams。
    fn test_params() -> RenderParams {
        RenderParams {
            width: 100,
            height: 100,
            ..RenderParams::default()
        }
    }

    #[test]
    fn backend_type_name() {
        assert_eq!(BackendType::Vello.name(), "Vello");
        assert_eq!(BackendType::Skia.name(), "Skia");
    }

    #[test]
    fn backend_type_debug() {
        let backends = [BackendType::Vello, BackendType::Skia];
        // 确保枚举可以 debug 格式化
        let formatted = format!("{:?}", backends);
        assert!(formatted.contains("Vello") || formatted.contains("Skia"));
    }

    #[test]
    fn available_backends_returns_at_least_one_with_features() {
        let available = RenderBackendFactory::available_backends();
        // 至少应该有一个后端可用（因为 feature 会启用至少一个）
        // 如果无 feature 测试运行，可能返回空列表，这是预期的
        if cfg!(feature = "skia-backend") {
            assert!(available.contains(&BackendType::Skia));
        }
    }

    #[test]
    fn available_backends_priority_order() {
        let available = RenderBackendFactory::available_backends();
        // 验证优先级顺序：Vello 优先于 Skia
        if available.len() >= 2 {
            assert_eq!(available[0], BackendType::Vello);
            assert_eq!(available[1], BackendType::Skia);
        }
    }

    #[test]
    fn create_backend_skia_succeeds_with_feature() {
        let params = test_params();
        let result = RenderBackendFactory::create_backend(BackendType::Skia, &params);
        if cfg!(feature = "skia-backend") {
            assert!(result.is_ok(), "expected Ok, got Err");
            let backend = result.unwrap();
            assert_eq!(backend.backend_name(), "Skia (CPU)");
        } else {
            // 无 skia-backend feature 时应返回 UnsupportedBackend
            match &result {
                Err(RenderError::UnsupportedBackend(msg)) => {
                    assert!(
                        msg.contains(&"skia-backend feature not enabled"),
                        "unexpected error message: {msg}"
                    );
                },
                Err(other) => panic!("expected UnsupportedBackend, got: {other:?}"),
                Ok(_) => panic!("expected Err without skia-backend feature"),
            }
        }
    }

    #[test]
    fn create_backend_vello_returns_error() {
        let params = test_params();
        let result = RenderBackendFactory::create_backend(BackendType::Vello, &params);
        // VelloBackend 尚未实现，总是返回错误
        assert!(result.is_err());
    }

    #[test]
    fn create_succeeds_with_skia_feature() {
        let params = test_params();
        let result = RenderBackendFactory::create(&params);
        if cfg!(feature = "skia-backend") {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    #[test]
    fn created_backend_implements_render_backend() {
        let params = test_params();
        if let Ok(mut backend) = RenderBackendFactory::create_backend(BackendType::Skia, &params) {
            // 验证 backend_name 返回非空字符串
            let name = backend.backend_name();
            assert!(!name.is_empty(), "backend name should not be empty");

            // 验证可以调用 render（空场景图不应 panic）
            let scene = SceneGraph::new(0);
            let render_result = backend.render(&scene, &params);
            // SkiaBackend 的空场景渲染应该成功
            assert!(
                render_result.is_ok(),
                "SkiaBackend render failed: {:?}",
                render_result
            );
        }
    }

    #[test]
    fn create_backend_vello_error_message() {
        let params = test_params();
        let result = RenderBackendFactory::create_backend(BackendType::Vello, &params);
        assert!(result.is_err());

        // 使用 match 或 is_err() 检查，避免 unwrap_err() 要求 Box<dyn RenderBackend>: Debug
        match &result {
            Err(RenderError::UnsupportedBackend(msg)) => {
                assert!(
                    msg.contains("R05"),
                    "error message should reference R05: {msg}"
                );
            },
            Err(other) => panic!("expected UnsupportedBackend, got: {other:?}"),
            Ok(_) => panic!("expected Err for VelloBackend"),
        }
    }

    /// 测试工厂创建的后端可以直接用于渲染。
    #[test]
    fn factory_backend_round_trip() {
        let params = test_params();
        if let Ok(mut backend) = RenderBackendFactory::create_backend(BackendType::Skia, &params) {
            let scene = SceneGraph::new(0);
            let result = backend.render(&scene, &params);
            assert!(result.is_ok(), "round-trip render failed: {:?}", result);
        }
    }

    /// 测试 BackendType 的 PartialEq 和 Eq。
    #[test]
    fn backend_type_equality() {
        assert_eq!(BackendType::Vello, BackendType::Vello);
        assert_eq!(BackendType::Skia, BackendType::Skia);
        assert_ne!(BackendType::Vello, BackendType::Skia);
    }

    /// 测试 BackendType 可用于 HashMap 键（需要 Hash + Eq）。
    #[test]
    fn backend_type_in_hashmap() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(BackendType::Vello, "vello");
        map.insert(BackendType::Skia, "skia");
        assert_eq!(map.get(&BackendType::Vello), Some(&"vello"));
        assert_eq!(map.get(&BackendType::Skia), Some(&"skia"));
    }
}
