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

/// 渲染后端抽象（D0 §3.5、D3 §5.1）。
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

    /// 更新纹理子区域（用于字形 Atlas 动态增长）。
    ///
    /// `x`, `y`, `width`, `height` 指定纹理内的子区域（像素坐标）。
    /// `data` 为 RGBA8 像素数据，长度必须为 `width * height * 4`。
    fn update_texture(
        &mut self,
        id: TextureId,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
    );

    /// 当前后端名称。
    fn backend_name(&self) -> &'static str;

    /// 后端是否可用（设备正常、surface 就绪）。
    fn is_available(&self) -> bool;

    /// 后端特性集。
    fn capabilities(&self) -> BackendCapabilities;
}

/// 后端特性描述（D3 §5.1）。
#[derive(Debug, Clone, PartialEq)]
pub struct BackendCapabilities {
    /// 是否支持多重采样抗锯齿。
    pub msaa: bool,
    /// 最大纹理尺寸（像素，宽或高的上限）。
    pub max_texture_size: u32,
    /// 是否支持 GPU 曲面细分。
    pub gpu_tessellation: bool,
    /// 是否支持高动态范围渲染。
    pub hdr: bool,
    /// 是否支持离屏渲染（不依赖窗口）。
    pub offscreen_rendering: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_capabilities_debug() {
        let caps = BackendCapabilities {
            msaa: true,
            max_texture_size: 16384,
            gpu_tessellation: true,
            hdr: false,
            offscreen_rendering: false,
        };
        let dbg = format!("{:?}", caps);
        assert!(dbg.contains("16384"));
    }

    #[test]
    fn backend_capabilities_clone_eq() {
        let a = BackendCapabilities {
            msaa: true,
            max_texture_size: 4096,
            gpu_tessellation: false,
            hdr: true,
            offscreen_rendering: true,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn render_error_device_lost_display() {
        let err = RenderError::DeviceLost("GPU 崩溃".into());
        let msg = err.to_string();
        assert!(msg.contains("GPU 设备不可用"));
    }

    #[test]
    fn render_error_surface_creation_failed_display() {
        let err = RenderError::SurfaceCreationFailed("无法创建 surface".into());
        let msg = err.to_string();
        assert!(msg.contains("表面创建失败"));
    }

    #[test]
    fn render_error_shader_compilation_failed_display() {
        let err = RenderError::ShaderCompilationFailed("shader 语法错误".into());
        let msg = err.to_string();
        assert!(msg.contains("着色器编译失败"));
    }

    #[test]
    fn render_error_render_failed_display() {
        let err = RenderError::RenderFailed("超时".into());
        let msg = err.to_string();
        assert!(msg.contains("渲染失败"));
    }

    #[test]
    fn render_error_texture_registration_failed_display() {
        let err = RenderError::TextureRegistrationFailed("格式不支持".into());
        let msg = err.to_string();
        assert!(msg.contains("纹理注册失败"));
    }

    #[test]
    fn render_error_no_available_backend_display() {
        let err = RenderError::NoAvailableBackend;
        let msg = err.to_string();
        assert!(msg.contains("无可用渲染后端"));
    }

    #[test]
    fn render_error_unsupported_backend_display() {
        let err = RenderError::UnsupportedBackend("OpenGL");
        let msg = err.to_string();
        assert!(msg.contains("后端不可用"));
        assert!(msg.contains("OpenGL"));
    }
}
