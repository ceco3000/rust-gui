//! 渲染模块——`rgui_render` 的 facade 层便捷重导出。
//!
//! 所有渲染功能委托给 [`rgui_render`]，本模块仅提供便利的重新导出。
//!
//! # 架构约束（D0 §3.5, D3 §5）
//!
//! facade 层仅通过 [`RenderBackend`](rgui_render::RenderBackend) trait 接口委托渲染，
//! 不持有 wgpu 设备或 surface 的直接引用。所有 GPU 资源管理封装在
//! [`VelloBackend`] 内部，符合 D8 R06 架构约束。

pub use rgui_render::{VelloBackend, encode_scene_to_vello};
