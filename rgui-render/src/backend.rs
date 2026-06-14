//! 渲染后端抽象——RenderBackend trait。
//!
//! 定义源自 D0 §3.5 和 D3 §5。

use crate::scene::SceneGraph;
use crate::texture::{TextureData, TextureFormat, TextureId};

/// 渲染错误类型。
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("GPU 设备不可用")]
    DeviceLost(String),
    #[error("表面创建失败：{0}")]
    SurfaceCreationFailed(String),
    #[error("着色器编译失败：{0}")]
    ShaderCompilationFailed(String),
    #[error("渲染失败：{0}")]
    RenderFailed(String),
    #[error("纹理注册失败：{0}")]
    TextureRegistrationFailed(String),
    /// 无可用渲染后端（所有后端均不可用）。
    #[error("无可用渲染后端")]
    NoAvailableBackend,
    /// 指定后端不可用（未编译或未实现）。
    #[error("后端不可用：{0}")]
    UnsupportedBackend(&'static str),
}

/// 渲染参数。
///
/// 定义源自 D3 §5.1。
#[derive(Clone, Debug)]
pub struct RenderParams {
    /// 逻辑像素密度比（DPI 缩放因子）。
    pub scale_factor: f64,
    /// 是否启用垂直同步。
    pub vsync: bool,
    /// 清除颜色（窗口背景色）。
    pub clear_color: Option<rgui_core::Color>,
    /// 渲染表面宽度（像素单位）。
    pub width: u32,
    /// 渲染表面高度（像素单位）。
    pub height: u32,
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            scale_factor: 1.0,
            vsync: true,
            clear_color: None,
            width: 1,
            height: 1,
        }
    }
}

/// 渲染后端抽象（D0 §3.5）。
///
/// 主实现：VelloBackend（需 `vello-backend` feature）
/// 预留实现：SkiaBackend（需 `skia-backend` feature）
pub trait RenderBackend: Send + Sync {
    /// 提交场景图并渲染。
    fn render(&mut self, scene: &SceneGraph, params: &RenderParams) -> Result<(), RenderError>;

    /// 注册纹理数据。
    fn register_texture(&mut self, data: &TextureData, format: TextureFormat) -> TextureId;

    /// 释放纹理。
    fn unregister_texture(&mut self, id: TextureId);

    /// 当前后端名称。
    fn backend_name(&self) -> &'static str;
}
