//! wgpu 渲染协调——单帧绘制、scene graph 提交、重绘脏标记（契约 §3.1 模块 3）。
//! D3 阶段 0：占位骨架。

use rgui_render::SceneGraph;

/// 渲染协调器（D3 占位）。
#[derive(Debug, Default)]
pub struct RenderCoord {
    /// 场景图。
    pub scene: Option<SceneGraph>,
}

impl RenderCoord {
    /// 构造空渲染协调器。
    pub fn new() -> Self {
        Self::default()
    }
}
